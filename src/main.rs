#![windows_subsystem = "windows"]

mod utils;
mod validation;

extern crate nalgebra_glm as glm;

use std::ffi::{CStr, CString};

use anyhow::{Error, Result};
use ash::version::{InstanceV1_0, InstanceV1_1, InstanceV1_2};
use ash::{version::EntryV1_0, vk};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::validation::ValidationInfo;

const APPLICATION_NAME: &str = "vrs";
const ENGINE_TITLE: &str = "ash";
const APPLICATION_VERSION: u32 = vk::make_version(1, 0, 0);
const ENGINE_VERSION: u32 = vk::make_version(1, 0, 0);

const VALIDATION: ValidationInfo = ValidationInfo {
    is_enabled: true,
    required_validation_layers: ["VK_LAYER_KHRONOS_validation"],
};

#[derive(Debug, Copy, Clone)]
struct QueueFamilyIndices {
    graphics_family: Option<u32>,
}

impl QueueFamilyIndices {
    fn is_complete(&self) -> bool {
        self.graphics_family.is_some()
    }
}

struct App {
    _entry: ash::Entry,
    instance: ash::Instance,

    surface_ext: ash::extensions::khr::Surface,
    debug_utils_ext: ash::extensions::ext::DebugUtils,

    surface: vk::SurfaceKHR,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl App {
    fn new() -> Result<(EventLoop<()>, Window, Self)> {
        let entry = ash::Entry::new()?;

        let event_loop = EventLoop::new();
        let window = Self::init_window(&event_loop)?;

        let mut debug_utils_messenger_create_info = validation::create_debug_messenger_create_info();

        let instance = Self::create_instance(&entry, &window, &mut debug_utils_messenger_create_info)?;

        let (debug_utils_ext, debug_messenger) =
            validation::setup_debug_utils(&entry, &instance, &debug_utils_messenger_create_info, &VALIDATION)?;

        let physical_device = Self::pick_physical_device(&instance)?;

        let surface_ext = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe { ash_window::create_surface(&entry, &instance, &window, None)? };
        log::debug!("created surface: {:?}", surface);

        Ok((
            event_loop,
            window,
            Self {
                _entry: entry,
                instance,
                surface_ext,
                debug_utils_ext,
                surface,
                debug_messenger,
            },
        ))
    }

    fn init_window(event_loop: &EventLoop<()>) -> Result<winit::window::Window> {
        let window = winit::window::WindowBuilder::new()
            .with_min_inner_size(LogicalSize::new(800, 600))
            .with_inner_size(LogicalSize::new(1024, 768))
            .build(event_loop)?;

        Ok(window)
    }

    fn create_instance(
        entry: &ash::Entry,
        window: &Window,
        debug_utils_messenger_create_info: &mut vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> Result<ash::Instance> {
        validation::check_validation_error_support(entry, &VALIDATION)?;

        let application_name = CString::new(APPLICATION_NAME)?;
        let engine_name = CString::new(ENGINE_TITLE)?;

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&application_name)
            .engine_name(&engine_name)
            .application_version(APPLICATION_VERSION)
            .engine_version(ENGINE_VERSION);

        //
        let mut required_extensions = ash_window::enumerate_required_extensions(window)?;
        required_extensions.push(ash::extensions::ext::DebugUtils::name());

        required_extensions.iter().for_each(|extension| {
            log::debug!("required extension: {:?}", extension);
        });

        let required_extensions = required_extensions.into_iter().map(CStr::as_ptr).collect::<Vec<_>>();

        //
        let required_layers = if VALIDATION.is_enabled {
            VALIDATION
                .required_validation_layers
                .iter()
                .map(|layer_name| unsafe { CStr::from_bytes_with_nul_unchecked(layer_name.as_bytes()) })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        required_layers.iter().for_each(|layer| {
            log::debug!("required layer: {:?}", layer);
        });

        let required_layers: Vec<*const i8> = required_layers.into_iter().map(CStr::as_ptr).collect::<Vec<_>>();

        //
        let mut instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&required_extensions)
            .enabled_layer_names(&required_layers);

        //
        if VALIDATION.is_enabled {
            instance_info = instance_info.push_next(debug_utils_messenger_create_info);
        }

        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        log::debug!("created instance");

        Ok(instance)
    }

    fn is_physical_device_suitable(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> bool {
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        //let device_features = unsafe { instance.get_physical_device_features(physical_device) };
        let device_queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let device_rt_properties =
            unsafe { ash::extensions::nv::RayTracing::get_properties(instance, physical_device) };

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
            return false;
        }

        log::debug!("{:#?}", device_rt_properties);

        let mut queue_family_indices = QueueFamilyIndices { graphics_family: None };

        let mut index = 0;
        for queue_family in device_queue_families.iter() {
            if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                queue_family_indices.graphics_family = Some(index);
            }

            if queue_family_indices.is_complete() {
                break;
            }

            index += 1;
        }

        queue_family_indices.is_complete()
    }

    fn pick_physical_device(instance: &ash::Instance) -> Result<vk::PhysicalDevice> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };

        let mut result = None;
        for &physical_device in physical_devices.iter() {
            if Self::is_physical_device_suitable(instance, physical_device) && result.is_none() {
                result = Some(physical_device);
            }
        }

        match result {
            Some(device) => Ok(device),
            None => Err(Error::msg("no suitable physical device found")),
        }
    }

    fn draw_frame(&mut self) {
        // drawing here
    }

    fn run(mut self, event_loop: EventLoop<()>, _window: Window) -> ! {
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::RedrawRequested(_) => {
                self.draw_frame();
            }
            _ => {}
        })
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            if VALIDATION.is_enabled {
                self.debug_utils_ext
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
                log::debug!("dropped debug utils messenger");
            }

            self.surface_ext.destroy_surface(self.surface, None);
            log::debug!("dropped surface");

            self.instance.destroy_instance(None);
            log::debug!("dropped instance");
        }
    }
}

fn run() -> Result<()> {
    let (event_loop, window, app) = App::new()?;
    app.run(event_loop, window)
}

fn main() {
    env_logger::init();

    if let Err(e) = run() {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
