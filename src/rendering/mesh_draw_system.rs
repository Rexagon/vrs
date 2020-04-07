use std::sync::Arc;

use vulkano::buffer::{BufferAccess, BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::device::Queue;
use vulkano::framebuffer::{RenderPassAbstract, Subpass};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};

pub struct MeshDrawSystem {
    queue: Arc<Queue>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
}

impl MeshDrawSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let pipeline = {
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
                    .depth_stencil_simple_depth()
                    .render_pass(subpass)
                    .build(queue.device().clone())
                    .unwrap(),
            ) as Arc<_>
        };

        Self { queue, pipeline }
    }

    pub fn draw<V>(&self, dynamic_state: &DynamicState, vertex_buffer: Arc<V>) -> AutoCommandBuffer
    where
        V: BufferAccess + Send + Sync + 'static,
    {
        AutoCommandBufferBuilder::secondary_graphics(
            self.queue.device().clone(),
            self.queue.family(),
            self.pipeline.clone().subpass(),
        )
        .unwrap()
        .draw(self.pipeline.clone(), dynamic_state, vec![vertex_buffer], (), ())
        .unwrap()
        .build()
        .unwrap()
    }

    pub fn create_simple_mesh(&self) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
        CpuAccessibleBuffer::from_iter(
            self.queue.device().clone(),
            BufferUsage::all(),
            false,
            [
                Vertex {
                    position: [-0.5, -0.25],
                },
                Vertex { position: [0.0, 0.5] },
                Vertex { position: [0.25, -0.1] },
            ]
            .iter()
            .cloned(),
        )
        .expect("Failed to create vertex buffer")
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

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
