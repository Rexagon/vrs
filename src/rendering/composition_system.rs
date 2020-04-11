use super::prelude::*;
use super::utils::*;

pub struct CompositionSystem {
    queue: Arc<Queue>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[ScreenVertex]>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

impl CompositionSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>, input: ComposingSystemInput) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
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

            let fragment_shader =
                fragment_shader::Shader::load(queue.device().clone()).expect("Failed to create fragment shader module");

            Arc::new(
                GraphicsPipeline::start()
                    .vertex_input_single_buffer::<ScreenVertex>()
                    .vertex_shader(vertex_shader.main_entry_point(), ())
                    .triangle_fan()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(fragment_shader.main_entry_point(), ())
                    .render_pass(subpass)
                    .build(queue.device().clone())
                    .unwrap(),
            )
        };

        let descriptor_set = input.into_descriptor_set(pipeline.as_ref());

        Self {
            queue,
            vertex_buffer,
            pipeline,
            descriptor_set,
        }
    }

    pub fn draw(&self, dynamic_state: &DynamicState) -> AutoCommandBuffer {
        AutoCommandBufferBuilder::secondary_graphics(
            self.queue.device().clone(),
            self.queue.family(),
            self.pipeline.clone().subpass(),
        )
        .unwrap()
        .draw(
            self.pipeline.clone(),
            dynamic_state,
            vec![self.vertex_buffer.clone()],
            self.descriptor_set.clone(),
            (),
        )
        .unwrap()
        .build()
        .unwrap()
    }
}

pub struct ComposingSystemInput {
    pub diffuse: Arc<AttachmentImage>,
    pub light: Arc<AttachmentImage>,
    pub depth: Arc<AttachmentImage>,
}

impl ComposingSystemInput {
    fn into_descriptor_set(
        self,
        pipeline: &(dyn GraphicsPipelineAbstract + Send + Sync),
    ) -> Arc<dyn DescriptorSet + Send + Sync> {
        let layout = pipeline.descriptor_set_layout(0).unwrap();
        Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_image(self.diffuse)
                .unwrap()
                .add_image(self.light)
                .unwrap()
                .add_image(self.depth)
                .unwrap()
                .build()
                .unwrap(),
        )
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/compose.frag"
    }
}
