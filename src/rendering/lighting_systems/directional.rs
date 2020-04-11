use crate::rendering::prelude::*;

use super::{LightingSystem, LightingSystemData, LightingSystemInput};

pub struct DirectionalLightingSystem(LightingSystemData);

impl DirectionalLightingSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>, input: LightingSystemInput) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let fragment_shader =
            fragment_shader::Shader::load(queue.device().clone()).expect("Failed to create fragment shader module");

        Self(Self::prepare_system(
            queue,
            subpass,
            input,
            fragment_shader.main_entry_point(),
            (),
        ))
    }

    pub fn draw(&self, dynamic_state: &DynamicState, color: [f32; 3], direction: [f32; 3]) -> AutoCommandBuffer {
        let push_constants = fragment_shader::ty::PushConstants {
            color: [color[0], color[1], color[2], 1.0],
            direction: [direction[0], direction[1], direction[2], 1.0],
        };

        self.create_command_buffer(dynamic_state, push_constants)
    }
}

impl LightingSystem for DirectionalLightingSystem {
    type ShaderPushConstants = fragment_shader::ty::PushConstants;

    fn inner(&self) -> &LightingSystemData {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut LightingSystemData {
        &mut self.0
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/directional.frag"
    }
}
