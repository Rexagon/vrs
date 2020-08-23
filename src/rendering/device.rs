use super::prelude::*;
use super::{utils, validation, Instance, Surface};

pub struct Device {
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    queues: Queues,
}

impl Device {
    pub fn new(instance: &Instance, surface: &Surface, is_validation_enabled: bool) -> Result<Self> {
        let (physical_device, queue_indices) = pick_physical_device(instance.handle(), surface)?;
        let memory_properties = unsafe { instance.handle().get_physical_device_memory_properties(physical_device) };

        let unique_queue_families = queue_indices.unique_families();

        let mut queue_create_infos = Vec::new();

        let queue_priorities = [1.0f32];
        for family in unique_queue_families.into_iter() {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(family)
                    .queue_priorities(&queue_priorities)
                    .build(),
            );
        }

        //
        let required_extensions = vec![
            ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::nv::RayTracing::name().as_ptr(),
        ];

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
        let device = unsafe {
            instance
                .handle()
                .create_device(physical_device, &device_create_info, None)?
        };
        let queues = Queues::new(&device, queue_indices)?;
        log::debug!("created logical device");

        Ok(Self {
            device,
            physical_device,
            memory_properties,
            queues,
        })
    }

    pub unsafe fn destroy(&self) {
        self.device.destroy_device(None);
        log::debug!("dropped logical device");
    }

    pub fn get_buffer_memory_requirements(&self, buffer: vk::Buffer) -> vk::MemoryRequirements {
        unsafe { self.device.get_buffer_memory_requirements(buffer) }
    }

    pub fn query_swapchain_support(&self, surface: &Surface) -> Result<SwapchainSupportInfo> {
        query_swapchain_support(surface, self.physical_device)
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;
        }
        Ok(())
    }

    pub fn find_supported_format(
        &self,
        instance: &Instance,
        candidate_formats: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format> {
        for &format in candidate_formats.iter() {
            let format_properties = unsafe {
                instance
                    .handle()
                    .get_physical_device_format_properties(self.physical_device, format)
            };

            if tiling == vk::ImageTiling::LINEAR && format_properties.linear_tiling_features.contains(features)
                || tiling == vk::ImageTiling::OPTIMAL && format_properties.optimal_tiling_features.contains(features)
            {
                return Ok(format);
            }
        }

        Err(Error::msg("failed to find supported format"))
    }

    #[inline]
    pub fn handle(&self) -> &ash::Device {
        &self.device
    }

    #[inline]
    pub fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.memory_properties
    }

    #[inline]
    pub fn queues(&self) -> &Queues {
        &self.queues
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwapchainSupportInfo {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub available_formats: Vec<vk::SurfaceFormatKHR>,
    pub available_present_modes: Vec<vk::PresentModeKHR>,
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

    fn unique_families(&self) -> HashSet<u32> {
        let mut result = HashSet::new();
        self.graphics_family.map(|idx| result.insert(idx));
        self.present_family.map(|idx| result.insert(idx));
        result
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Queues {
    pub graphics_queue: vk::Queue,
    pub graphics_queue_family: u32,
    pub present_queue: vk::Queue,
    pub present_queue_family: u32,
}

impl Queues {
    fn new(device: &ash::Device, indices: QueueFamilyIndices) -> Result<Self> {
        let graphics_queue_family = indices
            .graphics_family
            .ok_or_else(|| Error::msg("graphics family is not specified"))?;

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family, 0) };

        let present_queue_family = indices
            .present_family
            .ok_or_else(|| Error::msg("present family is not specified"))?;

        let present_queue = unsafe { device.get_device_queue(present_queue_family, 0) };

        Ok(Self {
            graphics_queue_family,
            graphics_queue,
            present_queue_family,
            present_queue,
        })
    }
}

fn pick_physical_device(
    instance: &ash::Instance,
    surface: &Surface,
) -> Result<(vk::PhysicalDevice, QueueFamilyIndices)> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    let mut result = None;
    for &physical_device in physical_devices.iter() {
        let indices = check_physical_device(instance, surface, physical_device)?;

        if indices.is_complete() && result.is_none() {
            result = Some((physical_device, indices));
        }
    }

    match result {
        Some((device, indices)) => Ok((device, indices)),
        None => Err(Error::msg("no suitable physical device found")),
    }
}

fn check_physical_device(
    instance: &ash::Instance,
    surface: &Surface,
    physical_device: vk::PhysicalDevice,
) -> Result<QueueFamilyIndices> {
    // check device properties
    let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };

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

    // check device extension support
    let device_extensions = unsafe { instance.enumerate_device_extension_properties(physical_device)? };

    let mut required_extensions = HashSet::new();
    required_extensions.insert(ash::extensions::khr::Swapchain::name());
    required_extensions.insert(ash::extensions::nv::RayTracing::name());

    for item in device_extensions {
        let extension_name = utils::from_vk_string_raw(&item.extension_name);
        required_extensions.remove(extension_name);
    }

    if !required_extensions.is_empty() {
        for item in required_extensions.into_iter() {
            log::debug!("extension {:?} is not supported by device", item);
        }
        return Ok(Default::default());
    }

    // check swapchain support
    let swapchain_support = query_swapchain_support(surface, physical_device)?;
    if swapchain_support.available_formats.is_empty() || swapchain_support.available_present_modes.is_empty() {
        return Ok(Default::default());
    }

    // find supported families
    let mut queue_family_indices = QueueFamilyIndices {
        graphics_family: None,
        present_family: None,
    };

    let device_queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

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

    // done
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
