use super::prelude::*;
use super::Device;

pub struct CommandPool {
    command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(device: &Device) -> Result<Self> {
        let command_pool_create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queues().graphics_queue_family);

        let command_pool = unsafe { device.handle().create_command_pool(&command_pool_create_info, None)? };
        log::debug!("created command pool {:?}", command_pool);

        Ok(Self { command_pool })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_command_pool(self.command_pool, None);
        log::debug!("dropped command pool {:?}", self.command_pool);
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.command_pool
    }
}
