use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::logical_device::LogicalDevice;

pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
}

impl Framebuffer {
    pub fn new(
        logical_device: &LogicalDevice,
        render_pass: vk::RenderPass,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Result<Self> {
        let framebuffer = create_framebuffer(logical_device, render_pass, image_view, extent)?;
        log::debug!("created framebuffer {:?}", framebuffer);

        Ok(Self { framebuffer })
    }

    #[allow(unused)]
    #[inline]
    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.handle().destroy_framebuffer(self.framebuffer, None);
        log::debug!("dropped framebuffer {:?}", self.framebuffer);
    }
}

fn create_framebuffer(
    logical_device: &LogicalDevice,
    render_pass: vk::RenderPass,
    image_view: vk::ImageView,
    extent: vk::Extent2D,
) -> Result<vk::Framebuffer> {
    let attachment = [image_view];

    let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&attachment)
        .width(extent.width)
        .height(extent.height)
        .layers(1);

    let framebuffer = unsafe {
        logical_device
            .handle()
            .create_framebuffer(&framebuffer_create_info, None)?
    };

    Ok(framebuffer)
}
