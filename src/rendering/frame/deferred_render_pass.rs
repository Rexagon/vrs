use crate::rendering::prelude::*;
use crate::rendering::Device;

pub struct DeferredRenderPass {
    render_pass: vk::RenderPass,
}

impl DeferredRenderPass {
    pub fn new(device: &Device, surface_format: vk::Format, depth_format: vk::Format) -> Result<Self> {
        // render pass
        let color_attachment = vk::AttachmentDescription::builder()
            .format(surface_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let depth_attachment = vk::AttachmentDescription::builder()
            .format(depth_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let render_pass_attachments = [color_attachment, depth_attachment];

        // subpasses
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let color_attachments = [color_attachment_ref];

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(&depth_attachment_ref)
            .build()];

        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .subpasses(&subpasses)
            .attachments(&render_pass_attachments);

        let render_pass = unsafe { device.handle().create_render_pass(&render_pass_create_info, None)? };
        log::debug!("created render pass {:?}", render_pass);

        Ok(Self { render_pass })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_render_pass(self.render_pass, None);
        log::debug!("dropped render pass {:?}", self.render_pass);
    }

    #[inline]
    pub fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }
}
