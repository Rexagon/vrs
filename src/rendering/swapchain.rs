use super::prelude::*;
use super::{Device, ImageView, Instance, Surface};

pub struct Swapchain {
    device: Arc<Device>,
    swapchain_ext: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<ImageView>,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(instance: &Instance, surface: &Surface, device: Arc<Device>, window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let size = [size.width, size.height];

        // select swapchain properties
        let swapchain_support = device.query_swapchain_support(surface)?;
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

        let queues = device.queues();

        let (image_sharing_mode, queue_family_indices) = if queues.graphics_queue_family != queues.present_queue_family
        {
            (
                vk::SharingMode::CONCURRENT,
                vec![queues.graphics_queue_family, queues.present_queue_family],
            )
        } else {
            (vk::SharingMode::EXCLUSIVE, Vec::new())
        };

        // create swapchain
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

        let swapchain_ext = ash::extensions::khr::Swapchain::new(instance.handle(), device.handle());
        let swapchain = unsafe { swapchain_ext.create_swapchain(&swapchain_create_info, None)? };
        log::debug!("created swapchain");

        let images = unsafe { swapchain_ext.get_swapchain_images(swapchain)? };

        // create image views
        let image_views = create_image_views(device.clone(), surface_format.format, &images)?;

        // done
        Ok(Self {
            device,
            swapchain_ext,
            swapchain,
            images,
            image_views,
            format: surface_format.format,
            extent,
        })
    }

    pub unsafe fn destroy(&self) {
        for image_view in self.image_views.iter() {
            image_view.destroy();
        }

        self.swapchain_ext.destroy_swapchain(self.swapchain, None);
        log::debug!("dropped swapchain");
    }

    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> Result<(u32, bool), vk::Result> {
        let (image_index, is_sub_optimal) = unsafe {
            self.swapchain_ext
                .acquire_next_image(self.swapchain, std::u64::MAX, semaphore, vk::Fence::null())?
        };

        Ok((image_index, is_sub_optimal))
    }

    pub fn present_image(&self, signal_semaphores: &[vk::Semaphore], image_index: u32) -> Result<bool> {
        let indices = [image_index];

        let swapchains = [self.swapchain];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&indices);

        let result = unsafe {
            self.swapchain_ext
                .queue_present(self.device.queues().graphics_queue, &present_info)
        };

        match result {
            Ok(is_sub_optimal) => Ok(is_sub_optimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }

    #[inline]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    #[inline]
    pub fn image_views(&self) -> &[ImageView] {
        &self.image_views
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    #[inline]
    pub fn image_count(&self) -> u32 {
        self.images.len() as u32
    }
}

fn choose_swapchain_format(available_formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for available_format in available_formats {
        if available_format.format == vk::Format::B8G8R8A8_SRGB
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return *available_format;
        }
    }

    *available_formats.first().unwrap()
}

fn choose_swapchain_present_mode(available_present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    for &available_present_mode in available_present_modes {
        // or vk::PresentModeKHR::MAILBOX
        if available_present_mode == vk::PresentModeKHR::FIFO {
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

fn create_image_views(device: Arc<Device>, surface_format: vk::Format, images: &[vk::Image]) -> Result<Vec<ImageView>> {
    images
        .iter()
        .map(|&image| ImageView::from_raw(device.clone(), image, surface_format, vk::ImageAspectFlags::COLOR, 1))
        .collect::<Result<_>>()
}
