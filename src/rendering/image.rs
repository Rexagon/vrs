#![allow(clippy::too_many_arguments)]

use super::prelude::*;
use crate::rendering::{Device, Memory};

pub struct Image {
    image: vk::Image,
    memory: Memory,
}

impl Image {
    pub fn new(
        device: &Device,
        size: [u32; 2],
        mip_levels: u32,
        samples: vk::SampleCountFlags,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        // create image
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(samples)
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .extent(vk::Extent3D {
                width: size[0],
                height: size[1],
                depth: 1,
            });

        let image = unsafe { device.handle().create_image(&image_create_info, None)? };
        log::debug!("created image {:?}", image);

        // allocate memroy
        let image_memory_requirements = unsafe { device.handle().get_image_memory_requirements(image) };

        let memory = Memory::new(device, &image_memory_requirements, required_memory_properties)?;

        // bind memory
        unsafe { device.handle().bind_image_memory(image, memory.handle(), 0)? };

        // done
        Ok(Self { image, memory })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_image(self.image, None);
        log::debug!("dropped image {:?}", self.image);

        self.memory.destroy(device);
    }

    #[inline]
    pub fn handle(&self) -> vk::Image {
        self.image
    }

    #[allow(unused)]
    #[inline]
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
}

pub struct ImageView {
    image_view: vk::ImageView,
}

impl ImageView {
    pub fn new(
        device: &Device,
        image: &Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        mip_levels: u32,
    ) -> Result<Self> {
        Self::from_raw(device, image.handle(), format, aspect_flags, mip_levels)
    }

    pub fn from_raw(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        mip_levels: u32,
    ) -> Result<Self> {
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image(image);

        let image_view = unsafe { device.handle().create_image_view(&image_view_create_info, None)? };
        log::debug!("created image view {:?}", image_view);

        Ok(Self { image_view })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_image_view(self.image_view, None);
        log::debug!("dropped image view {:?}", self.image_view);
    }

    #[inline]
    pub fn handle(&self) -> vk::ImageView {
        self.image_view
    }
}
