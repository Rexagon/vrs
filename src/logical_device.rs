use anyhow::{Error, Result};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::surface::Surface;
use crate::swapchain::SwapchainSupportInfo;
use crate::utils;
use crate::validation;

pub struct LogicalDevice {
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
}

impl LogicalDevice {
    pub fn new(instance: &ash::Instance, surface: &Surface, is_validation_enabled: bool) -> Result<Self> {
        let (physical_device, queue_indices) = pick_physical_device(instance, surface)?;
        let (device, graphics_queue) =
            create_logical_device(instance, physical_device, queue_indices, is_validation_enabled)?;
        log::debug!("created logical device");

        Ok(Self {
            device,
            physical_device,
            graphics_queue,
        })
    }

    #[allow(unused)]
    #[inline]
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    #[allow(unused)]
    #[inline]
    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    #[allow(unused)]
    #[inline]
    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub unsafe fn destroy(&self) {
        self.device.destroy_device(None);
        log::debug!("dropped logical device");
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_indices: QueueFamilyIndices,
    is_validation_enabled: bool,
) -> Result<(ash::Device, vk::Queue)> {
    let queue_priorities = [1.0f32];
    let mut queue_create_infos = Vec::new();
    queue_create_infos.push(
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_indices.graphics_family.unwrap() as u32)
            .queue_priorities(&queue_priorities)
            .build(),
    );

    //
    let required_extensions = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

    //
    let required_layers = if is_validation_enabled {
        validation::required_layers()
    } else {
        &[]
    };

    let required_layers = utils::as_ptr_vec(&required_layers);

    //
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&required_extensions)
        .enabled_layer_names(&required_layers);

    //
    let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let graphics_queue = unsafe { device.get_device_queue(queue_indices.graphics_family.unwrap() as u32, 0) };

    Ok((device, graphics_queue))
}

fn pick_physical_device(
    instance: &ash::Instance,
    surface: &Surface,
) -> Result<(vk::PhysicalDevice, QueueFamilyIndices)> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    let mut result = None;
    for &physical_device in physical_devices.iter() {
        let queue_indices = check_physical_device(instance, surface, physical_device)?;

        if queue_indices.is_complete() && result.is_none() {
            result = Some((physical_device, queue_indices));
        }
    }

    match result {
        Some(result) => Ok(result),
        None => Err(Error::msg("no suitable physical device found")),
    }
}

fn check_physical_device(
    instance: &ash::Instance,
    surface: &Surface,
    physical_device: vk::PhysicalDevice,
) -> Result<QueueFamilyIndices> {
    let mut queue_family_indices = QueueFamilyIndices {
        graphics_family: None,
        present_family: None,
    };

    let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
    //let device_features = unsafe { instance.get_physical_device_features(physical_device) };
    let device_queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let device_rt_properties = unsafe { ash::extensions::nv::RayTracing::get_properties(instance, physical_device) };

    let device_name = utils::from_vk_string(&device_properties.device_name);

    let device_type = match device_properties.device_type {
        vk::PhysicalDeviceType::CPU => "cpu",
        vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated GPU",
        vk::PhysicalDeviceType::DISCRETE_GPU => "discrete GPU",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual GPU",
        vk::PhysicalDeviceType::OTHER => "unknown",
        _ => unreachable!(),
    };

    log::debug!(
        "found device: {}, id: {}, type: {}",
        device_name,
        device_properties.device_id,
        device_type
    );

    let major_version = vk::version_major(device_properties.api_version);
    let minor_version = vk::version_minor(device_properties.api_version);
    let patch_version = vk::version_patch(device_properties.api_version);

    log::debug!(
        "supperted API version: {}.{}.{}",
        major_version,
        minor_version,
        patch_version
    );

    if device_rt_properties.max_geometry_count == 0 {
        log::warn!("ray tracing is not supported by {}", device_name);
        return Ok(queue_family_indices);
    }

    log::debug!("{:#?}", device_rt_properties);

    for (index, queue_family) in device_queue_families.iter().enumerate() {
        if queue_family.queue_count == 0 {
            continue;
        }

        if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            queue_family_indices.graphics_family = Some(index as u32);
        }

        let is_present_support = unsafe {
            surface
                .ext()
                .get_physical_device_surface_support(physical_device, index as u32, surface.handle())?
        };

        if is_present_support {
            queue_family_indices.present_family = Some(index as u32);
        }

        if queue_family_indices.is_complete() {
            break;
        }
    }

    Ok(queue_family_indices)
}

fn query_swapchain_support(surface: &Surface, physical_device: vk::PhysicalDevice) -> Result<SwapchainSupportInfo> {
    let ext = surface.ext();
    let surface = surface.handle();

    let capabilities = unsafe { ext.get_physical_device_surface_capabilities(physical_device, surface)? };
    let available_formats = unsafe { ext.get_physical_device_surface_formats(physical_device, surface)? };
    let available_present_modes = unsafe { ext.get_physical_device_surface_present_modes(physical_device, surface)? };

    Ok(SwapchainSupportInfo {
        capabilities,
        available_formats,
        available_present_modes,
    })
}
