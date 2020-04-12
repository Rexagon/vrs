use vulkano::buffer::{BufferAccess, CpuBufferPool};

use crate::rendering::prelude::*;

pub struct MeshDrawSystem {
    queue: Arc<Queue>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    world_uniform_buffer_pool: CpuBufferPool<vertex_shader::ty::WorldData>,
    world_descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

impl MeshDrawSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>, world_state: &WorldState) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync> = {
            let vertex_shader =
                vertex_shader::Shader::load(queue.device().clone()).expect("Failed to create vertex shader module");
            let fragment_shader =
                fragment_shader::Shader::load(queue.device().clone()).expect("Failed to create fragment shader module");

            Arc::new(
                GraphicsPipeline::start()
                    .vertex_input_single_buffer::<Vertex>()
                    .vertex_shader(vertex_shader.main_entry_point(), ())
                    .triangle_list()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(fragment_shader.main_entry_point(), ())
                    .depth_stencil(DepthStencil {
                        depth_compare: Compare::Less,
                        depth_write: true,
                        depth_bounds_test: DepthBounds::Disabled,
                        stencil_front: Stencil {
                            compare: Compare::Always,
                            pass_op: StencilOp::Replace,
                            fail_op: StencilOp::Replace,
                            depth_fail_op: StencilOp::Replace,
                            compare_mask: Some(0x80),
                            write_mask: Some(0xff),
                            reference: Some(0x80),
                        },
                        stencil_back: Stencil {
                            compare: Compare::Always,
                            pass_op: StencilOp::Replace,
                            fail_op: StencilOp::Keep,
                            depth_fail_op: StencilOp::Keep,
                            compare_mask: Some(0x80),
                            write_mask: Some(0xff),
                            reference: Some(0x80),
                        },
                    })
                    .render_pass(subpass)
                    .build(queue.device().clone())
                    .unwrap(),
            ) as Arc<_>
        };

        let mut world_uniform_buffer_pool =
            CpuBufferPool::<vertex_shader::ty::WorldData>::new(queue.device().clone(), BufferUsage::all());

        let world_descriptor_set = world_state
            .clone()
            .into_descriptor_set(pipeline.as_ref(), &mut world_uniform_buffer_pool);

        Self {
            queue,
            pipeline,
            world_uniform_buffer_pool,
            world_descriptor_set,
        }
    }

    pub fn set_world_state(&mut self, world_state: &WorldState) {
        self.world_descriptor_set = world_state
            .clone()
            .into_descriptor_set(self.pipeline.as_ref(), &mut self.world_uniform_buffer_pool);
    }

    pub fn draw<V>(
        &self,
        dynamic_state: &DynamicState,
        vertex_buffer: Arc<V>,
        index_buffer: Arc<CpuAccessibleBuffer<[u16]>>,
        mesh_state: &MeshState,
    ) -> AutoCommandBuffer
    where
        V: BufferAccess + Send + Sync + 'static,
    {
        let push_constants: vertex_shader::ty::MeshData = mesh_state.into();

        AutoCommandBufferBuilder::secondary_graphics(
            self.queue.device().clone(),
            self.queue.family(),
            self.pipeline.clone().subpass(),
        )
        .unwrap()
        .draw_indexed(
            self.pipeline.clone(),
            dynamic_state,
            vec![vertex_buffer],
            index_buffer,
            self.world_descriptor_set.clone(),
            push_constants,
        )
        .unwrap()
        .build()
        .unwrap()
    }

    pub fn create_simple_mesh(&self) -> (Arc<CpuAccessibleBuffer<[Vertex]>>, Arc<CpuAccessibleBuffer<[u16]>>) {
        // TODO: move into separate model loading system

        let file = std::fs::File::open("./models/monkey.glb").unwrap();
        let reader = std::io::BufReader::new(file);
        let gltf = gltf::Gltf::from_reader(reader).unwrap();

        let mesh = gltf.meshes().next().unwrap();
        let primitive = mesh.primitives().next().unwrap();

        let buffer = gltf.blob.as_ref().unwrap();

        let reader = primitive.reader(|_| Some(buffer));

        let vertex_buffer = CpuAccessibleBuffer::from_iter(self.queue.device().clone(), BufferUsage::all(), false, {
            reader
                .read_positions()
                .and_then(|positions_iter| reader.read_normals().map(|normals_iter| (positions_iter, normals_iter)))
                .map(|(positions_iter, normals_iter)| {
                    positions_iter.zip(normals_iter).map(|(position, normal)| Vertex {
                        position: [position[0], position[2], position[1]],
                        normal: [normal[0], normal[2], normal[1]],
                    })
                })
                .unwrap()
        })
        .expect("Failed to create vertex buffer");

        let index_buffer = CpuAccessibleBuffer::from_iter(
            self.queue.device().clone(),
            BufferUsage::all(),
            false,
            match reader.read_indices().unwrap() {
                gltf::mesh::util::ReadIndices::U8(iter) => itertools::Either::Left(iter.map(|index| index as u16)),
                gltf::mesh::util::ReadIndices::U16(iter) => itertools::Either::Right(iter),
                gltf::mesh::util::ReadIndices::U32(_) => panic!("Not yet"),
            },
        )
        .expect("Failed to create index buffer");

        (vertex_buffer, index_buffer)
    }
}

#[derive(Clone)]
pub struct WorldState {
    view: glm::Mat4,
    projection: glm::Mat4,
}

impl WorldState {
    pub fn set_view(&mut self, view: glm::Mat4) {
        self.view = view;
    }

    pub fn set_projection(&mut self, projection: glm::Mat4) {
        self.projection = projection;
    }

    pub fn into_descriptor_set(
        self,
        pipeline: &(dyn GraphicsPipelineAbstract + Send + Sync),
        uniform_buffer_pool: &mut CpuBufferPool<vertex_shader::ty::WorldData>,
    ) -> Arc<dyn DescriptorSet + Send + Sync> {
        let uniform_data = vertex_shader::ty::WorldData {
            view: self.view.into(),
            projection: self.projection.into(),
        };

        let uniform_buffer = uniform_buffer_pool.next(uniform_data).unwrap();
        let layout = pipeline.descriptor_set_layout(0).unwrap();
        Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_buffer(uniform_buffer)
                .unwrap()
                .build()
                .unwrap(),
        )
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            view: glm::Mat4::identity(),
            projection: glm::Mat4::identity(),
        }
    }
}

#[derive(Clone)]
pub struct MeshState {
    pub transform: glm::Mat4,
}

impl From<&MeshState> for vertex_shader::ty::MeshData {
    fn from(data: &MeshState) -> Self {
        Self {
            transform: data.transform.clone().into(),
        }
    }
}

impl Default for MeshState {
    fn default() -> Self {
        Self {
            transform: glm::Mat4::identity(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, normal);

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/mesh.vert"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/mesh.frag"
    }
}
