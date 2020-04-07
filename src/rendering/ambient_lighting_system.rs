use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::Queue;
use vulkano::framebuffer::{RenderPassAbstract, Subpass};
use vulkano::image::ImageViewAccess;
use vulkano::pipeline::blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};

pub struct AmbientLightingSystem {
    queue: Arc<Queue>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[ScreenVertex]>>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
}

impl AmbientLightingSystem {
    pub fn new<R>(queue: Arc<Queue>, subpass: Subpass<R>) -> Self
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            queue.device().clone(),
            BufferUsage::all(),
            false,
            [
                ScreenVertex { position: [-1.0, -1.0] },
                ScreenVertex { position: [1.0, -1.0] },
                ScreenVertex { position: [1.0, 1.0] },
                ScreenVertex { position: [-1.0, 1.0] },
            ]
            .iter()
            .cloned(),
        )
        .expect("Failed to create vertex buffer");

        let pipeline = {
            let vertex_shader =
                vertex_shader::Shader::load(queue.device().clone()).expect("Failed to create vertex shader module");
            let fragment_shader =
                fragment_shader::Shader::load(queue.device().clone()).expect("Failed to create fragment shader module");

            Arc::new(
                GraphicsPipeline::start()
                    .vertex_input_single_buffer::<ScreenVertex>()
                    .vertex_shader(vertex_shader.main_entry_point(), ())
                    .triangle_fan()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(fragment_shader.main_entry_point(), ())
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

        Self {
            queue,
            vertex_buffer,
            pipeline,
        }
    }

    pub fn draw<C>(&self, dynamic_state: &DynamicState, color_input: C, ambient_color: [f32; 3]) -> AutoCommandBuffer
    where
        C: ImageViewAccess + Send + Sync + 'static,
    {
        let push_constants = fragment_shader::ty::PushConstants {
            color: [ambient_color[0], ambient_color[1], ambient_color[2], 1.0],
        };

        let layout = self.pipeline.descriptor_set_layout(0).unwrap();
        let descriptor_set = PersistentDescriptorSet::start(layout.clone())
            .add_image(color_input)
            .unwrap()
            .build()
            .unwrap();

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
            descriptor_set,
            push_constants,
        )
        .unwrap()
        .build()
        .unwrap()
    }
}

#[derive(Default, Debug, Clone)]
struct ScreenVertex {
    position: [f32; 2],
}
vulkano::impl_vertex!(ScreenVertex, position);

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/screen.vert"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/ambient.frag"
    }
}
