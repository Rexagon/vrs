use super::prelude::*;
use super::Device;

pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(
        device: &Device,
        render_pass: vk::RenderPass,
        attachments: &[vk::ImageView],
        extent: vk::Extent2D,
    ) -> Result<Self> {
        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer = unsafe { device.handle().create_framebuffer(&framebuffer_create_info, None)? };
        log::debug!("created framebuffer {:?}", framebuffer);

        Ok(Self { framebuffer })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_framebuffer(self.framebuffer, None);
        log::debug!("dropped framebuffer {:?}", self.framebuffer);
    }

    #[inline]
    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}
