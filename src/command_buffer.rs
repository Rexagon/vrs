use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::logical_device::LogicalDevice;

pub struct CommandPool {
    command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(logical_device: &LogicalDevice) -> Result<Self> {
        let command_pool_create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(logical_device.queues().graphics_queue_family);

        let command_pool = unsafe {
            logical_device
                .handle()
                .create_command_pool(&command_pool_create_info, None)?
        };
        log::debug!("created command pool {:?}", command_pool);

        Ok(Self { command_pool })
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.command_pool
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.handle().destroy_command_pool(self.command_pool, None);
        log::debug!("dropped command pool {:?}", self.command_pool);
    }
}
