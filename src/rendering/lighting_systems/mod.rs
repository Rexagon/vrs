pub mod ambient;
pub mod directional;

pub use ambient::*;
pub use directional::*;

use super::prelude::*;

trait ScreenQuadExt {
    fn build_lighting_graphics_pipeline<R, Fs, C>(
        &self,
        queue: Arc<Queue>,
        subpass: Subpass<R>,
        fragment_shader: Fs,
        specialization_constants: C,
    ) -> Arc<dyn GraphicsPipelineAbstract + Send + Sync>
    where
        R: RenderPassAbstract + Send + Sync + 'static,
        Fs: GraphicsEntryPointAbstract<SpecializationConstants = C>,
        Fs::PipelineLayout: Clone + Send + Sync + 'static,
        C: SpecializationConstants;
}

impl ScreenQuadExt for crate::rendering::screen_quad::ScreenQuad {
    fn build_lighting_graphics_pipeline<R, Fs, C>(
        &self,
        queue: Arc<Queue>,
        subpass: Subpass<R>,
        fragment_shader: Fs,
        specialization_constants: C,
    ) -> Arc<dyn GraphicsPipelineAbstract + Send + Sync>
    where
        R: RenderPassAbstract + Send + Sync + 'static,
        Fs: GraphicsEntryPointAbstract<SpecializationConstants = C>,
        Fs::PipelineLayout: Clone + Send + Sync + 'static,
        C: SpecializationConstants,
    {
        Arc::new(
            self.start_graphics_pipeline()
                .fragment_shader(fragment_shader, specialization_constants)
                .blend_collective(LIGHTING_PIPELINE_ATTACHMENT_BLEND)
                .render_pass(subpass)
                .build(queue.device().clone())
                .unwrap(),
        )
    }
}

const LIGHTING_PIPELINE_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
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
};
