use crate::rendering::prelude::*;
use crate::rendering::screen_quad::*;

use super::ScreenQuadExt;

pub struct AmbientLightingSystem {
    queue: Arc<Queue>,
    vertex_buffer: Arc<ScreenQuadVertexBuffer>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
}

impl AmbientLightingSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>, screen_quad: &ScreenQuad) -> Self
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

        Self {
            queue,
            vertex_buffer,
            pipeline,
        }
    }

    pub fn draw(&self, dynamic_state: &DynamicState, ambient_color: [f32; 3]) -> AutoCommandBuffer {
        let push_constants = fragment_shader::ty::LightParameters {
            color: [ambient_color[0], ambient_color[1], ambient_color[2], 1.0],
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
            (),
            push_constants,
        )
        .unwrap()
        .build()
        .unwrap()
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/ambient.frag"
    }
}
