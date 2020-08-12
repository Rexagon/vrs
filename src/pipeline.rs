use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::logical_device::LogicalDevice;

pub struct PipelineCache {
    pipeline_cache: vk::PipelineCache,
}

impl PipelineCache {
    pub fn new(logical_device: &LogicalDevice) -> Result<Self> {
        let pipeline_cache_create_info = vk::PipelineCacheCreateInfo::builder();

        let pipeline_cache = unsafe {
            logical_device
                .handle()
                .create_pipeline_cache(&pipeline_cache_create_info, None)?
        };
        log::debug!("created pipeline cache {:?}", pipeline_cache);

        Ok(Self { pipeline_cache })
    }

    #[inline]
    pub fn handle(&self) -> vk::PipelineCache {
        self.pipeline_cache
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device
            .handle()
            .destroy_pipeline_cache(self.pipeline_cache, None);
        log::debug!("dropped pipeline cache {:?}", self.pipeline_cache);
    }
}
