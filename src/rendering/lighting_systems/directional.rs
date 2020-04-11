use crate::rendering::prelude::*;
use crate::rendering::screen_quad::*;
use crate::rendering::utils::IntoDescriptorSet;

use super::ScreenQuadExt;

pub struct DirectionalLightingSystem {
    queue: Arc<Queue>,
    vertex_buffer: Arc<ScreenQuadVertexBuffer>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

impl DirectionalLightingSystem {
    pub fn new<R>(
        queue: Arc<Queue>,
        subpass: Subpass<R>,
        screen_quad: &ScreenQuad,
        input: DirectionalLightingSystemInput,
    ) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let fragment_shader =
            fragment_shader::Shader::load(queue.device().clone()).expect("Failed to create fragment shader module");

        let vertex_buffer = screen_quad.vertex_buffer();
        let pipeline = screen_quad.build_lighting_graphics_pipeline(
            queue.clone(),
            subpass,
            fragment_shader.main_entry_point(),
            (),
        );

        let descriptor_set = input.into_descriptor_set(pipeline.as_ref());

        Self {
            queue,
            vertex_buffer,
            pipeline,
            descriptor_set,
        }
    }

    pub fn update_input(&mut self, input: DirectionalLightingSystemInput) {
        self.descriptor_set = input.into_descriptor_set(self.pipeline.as_ref());
    }

    pub fn draw(&self, dynamic_state: &DynamicState, color: [f32; 3], direction: [f32; 3]) -> AutoCommandBuffer {
        let push_constants = fragment_shader::ty::LightParameters {
            color: [color[0], color[1], color[2], 1.0],
            direction: [direction[0], direction[1], direction[2], 1.0],
        };

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
            push_constants,
        )
        .unwrap()
        .build()
        .unwrap()
    }
}

pub struct DirectionalLightingSystemInput {
    pub diffuse: Arc<AttachmentImage>,
    pub normals: Arc<AttachmentImage>,
}

impl IntoDescriptorSet for DirectionalLightingSystemInput {
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

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/directional.frag"
    }
}
