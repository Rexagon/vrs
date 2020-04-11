pub mod ambient;
pub mod directional;

pub use ambient::*;
pub use directional::*;

use super::prelude::*;
use super::utils::*;

trait LightingSystem: Sized {
    type ShaderPushConstants;

    fn update_input(&mut self, input: LightingSystemInput) {
        let inner = self.inner_mut();
        inner.descriptor_set = input.into_descriptor_set(inner.pipeline.as_ref());
    }

    fn create_command_buffer(
        &self,
        dynamic_state: &DynamicState,
        push_constants: Self::ShaderPushConstants,
    ) -> AutoCommandBuffer {
        let inner = self.inner();

        AutoCommandBufferBuilder::secondary_graphics(
            inner.queue.device().clone(),
            inner.queue.family(),
            inner.pipeline.clone().subpass(),
        )
        .unwrap()
        .draw(
            inner.pipeline.clone(),
            dynamic_state,
            vec![inner.vertex_buffer.clone()],
            inner.descriptor_set.clone(),
            push_constants,
        )
        .unwrap()
        .build()
        .unwrap()
    }

    fn prepare_system<R, Fs, FsConstants>(
        queue: Arc<Queue>,
        subpass: Subpass<R>,
        input: LightingSystemInput,
        fragment_shader: Fs,
        fragment_shader_constants: FsConstants,
    ) -> LightingSystemData
    where
        R: RenderPassAbstract + Send + Sync + 'static,
        Fs: GraphicsEntryPointAbstract<SpecializationConstants = FsConstants>,
        Fs::PipelineLayout: Clone + Send + Sync + 'static,
        FsConstants: SpecializationConstants,
    {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            queue.device().clone(),
            BufferUsage::all(),
            false,
            ScreenVertex::quad().iter().cloned(),
        )
        .expect("Failed to create vertex buffer");

        let pipeline = {
            let vertex_shader = screen_vertex_shader::Shader::load(queue.device().clone())
                .expect("Failed to create vertex shader module");

            Arc::new(
                GraphicsPipeline::start()
                    .vertex_input_single_buffer::<ScreenVertex>()
                    .vertex_shader(vertex_shader.main_entry_point(), ())
                    .triangle_fan()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(fragment_shader, fragment_shader_constants)
                    .blend_collective(AttachmentBlend {
                        enabled: true,
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                        mask_red: true,
                        mask_green: true,
                        mask_blue: true,
                        mask_alpha: true,
                    })
                    .render_pass(subpass)
                    .build(queue.device().clone())
                    .unwrap(),
            )
        };

        let descriptor_set = input.into_descriptor_set(pipeline.as_ref());

        LightingSystemData {
            queue,
            vertex_buffer,
            pipeline,
            descriptor_set,
        }
    }

    fn inner(&self) -> &LightingSystemData;
    fn inner_mut(&mut self) -> &mut LightingSystemData;
}

pub struct LightingSystemData {
    queue: Arc<Queue>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[ScreenVertex]>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

pub struct LightingSystemInput {
    pub diffuse: Arc<AttachmentImage>,
    pub normals: Arc<AttachmentImage>,
}

impl LightingSystemInput {
    fn into_descriptor_set(
        self,
        pipeline: &(dyn GraphicsPipelineAbstract + Send + Sync),
    ) -> Arc<dyn DescriptorSet + Send + Sync> {
        let layout = pipeline.descriptor_set_layout(0).unwrap();
        Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_image(self.diffuse)
                .unwrap()
                .add_image(self.normals)
                .unwrap()
                .build()
                .unwrap(),
        )
    }
}
