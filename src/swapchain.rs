use anyhow::{Error, Result};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::logical_device::LogicalDevice;
use crate::surface::Surface;

pub struct Swapchain {
    swapchain_ext: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(instance: &ash::Instance, surface: &Surface, logical_device: &LogicalDevice) -> Result<Self> {}
}

pub struct SwapchainSupportInfo {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub available_formats: Vec<vk::SurfaceFormatKHR>,
    pub available_present_modes: Vec<vk::PresentModeKHR>,
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
                size.0,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: num::clamp(
                size.1,
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}
