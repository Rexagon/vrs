use crate::rendering::prelude::*;

use super::{LightingSystem, LightingSystemData, LightingSystemInput};

pub struct AmbientLightingSystem(LightingSystemData);

impl AmbientLightingSystem {
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

    pub fn draw(&self, dynamic_state: &DynamicState, ambient_color: [f32; 3]) -> AutoCommandBuffer {
        let push_constants = fragment_shader::ty::PushConstants {
            color: [ambient_color[0], ambient_color[1], ambient_color[2], 1.0],
        };

        self.create_command_buffer(dynamic_state, push_constants)
    }
}

impl LightingSystem for AmbientLightingSystem {
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
        path: "shaders/ambient.frag"
    }
}
