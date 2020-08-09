use anyhow::{Error, Result};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::logical_device::LogicalDevice;
use crate::surface::Surface;

pub struct Swapchain {
    swapchain_ext: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(instance: &ash::Instance, surface: &Surface, logical_device: &LogicalDevice) -> Result<Self> {
        let (swapchain_ext, swapchain, format, extent) =
            create_swapchain(instance, surface, logical_device, [800, 600])?;
        log::debug!("created swapchain");

        let images = unsafe { swapchain_ext.get_swapchain_images(swapchain)? };

        let image_views = create_image_views(logical_device.device(), format, &images)?;

        Ok(Self {
            swapchain_ext,
            swapchain,
            images,
            image_views,
            format,
            extent,
        })
    }

    #[inline]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        destroy_image_views(logical_device.device(), &self.image_views);

        self.swapchain_ext.destroy_swapchain(self.swapchain, None);
        log::debug!("dropped swapchain");
    }
}

fn create_swapchain(
    instance: &ash::Instance,
    surface: &Surface,
    logical_device: &LogicalDevice,
    size: [u32; 2],
) -> Result<(
    ash::extensions::khr::Swapchain,
    vk::SwapchainKHR,
    vk::Format,
    vk::Extent2D,
)> {
    let swapchain_support = logical_device.swapchain_support();
    let surface_format = choose_swapchain_format(&swapchain_support.available_formats);
    let present_mode = choose_swapchain_present_mode(&swapchain_support.available_present_modes);
    let extent = choose_swapchain_extent(&swapchain_support.capabilities, size);

    // select image count
    let image_count = swapchain_support.capabilities.min_image_count + 1;
    let image_count = if swapchain_support.capabilities.max_image_count > 0 {
        std::cmp::min(image_count, swapchain_support.capabilities.max_image_count)
    } else {
        image_count
    };

    let queues = logical_device.queues();

    let (image_sharing_mode, queue_family_indices) = if queues.graphics_queue_family != queues.present_queue_family {
        (
            vk::SharingMode::CONCURRENT,
            vec![queues.graphics_queue_family, queues.present_queue_family],
        )
    } else {
        (vk::SharingMode::EXCLUSIVE, Vec::new())
    };

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface.handle())
        .min_image_count(image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(extent)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(swapchain_support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);

    let swapchain_ext = ash::extensions::khr::Swapchain::new(instance, logical_device.device());
    let swapchain = unsafe { swapchain_ext.create_swapchain(&swapchain_create_info, None)? };

    Ok((swapchain_ext, swapchain, surface_format.format, extent))
}

fn choose_swapchain_format(available_formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for available_format in available_formats {
        if available_format.format == vk::Format::B8G8R8A8_SRGB
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return available_format.clone();
        }
    }

    return available_formats.first().unwrap().clone();
}

fn choose_swapchain_present_mode(available_present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    for &available_present_mode in available_present_modes {
        if available_present_mode == vk::PresentModeKHR::MAILBOX {
            return available_present_mode;
        }
    }

    vk::PresentModeKHR::FIFO
}

fn choose_swapchain_extent(capabilities: &vk::SurfaceCapabilitiesKHR, size: [u32; 2]) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: num::clamp(
                size[0],
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: num::clamp(
                size[1],
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

pub fn create_image_views(
    device: &ash::Device,
    surface_format: vk::Format,
    images: &[vk::Image],
) -> Result<Vec<vk::ImageView>> {
    let mut result = Vec::with_capacity(images.len());

    for &image in images.iter() {
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(surface_format)
            .components(
                vk::ComponentMapping::builder()
                    .r(vk::ComponentSwizzle::IDENTITY)
                    .g(vk::ComponentSwizzle::IDENTITY)
                    .b(vk::ComponentSwizzle::IDENTITY)
                    .a(vk::ComponentSwizzle::IDENTITY)
                    .build(),
            )
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )
            .image(image);

        let image_view = unsafe { device.create_image_view(&image_view_create_info, None)? };
        log::debug!("created image view {:?}", image_view);

        result.push(image_view);
    }

    Ok(result)
}

pub unsafe fn destroy_image_views(device: &ash::Device, image_views: &[vk::ImageView]) {
    for &image_view in image_views.iter() {
        device.destroy_image_view(image_view, None);
        log::debug!("dropped image view {:?}", image_view);
    }
}
