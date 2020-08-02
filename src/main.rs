#![windows_subsystem = "windows"]

extern crate nalgebra_glm as glm;

use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;

use anyhow::{Error, Result};
use ash::version::InstanceV1_0;
use ash::{version::EntryV1_0, vk};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

const APPLICATION_NAME: &str = "vrs";
const ENGINE_TITLE: &str = "ash";
const APPLICATION_VERSION: u32 = vk::make_version(1, 0, 0);
const ENGINE_VERSION: u32 = vk::make_version(1, 0, 0);

const VALIDATION: ValidationInfo = ValidationInfo {
    is_enabled: true,
    required_validation_layers: ["VK_LAYER_KHRONOS_validation"],
};

fn from_vk_string(raw_string_array: &[c_char]) -> Result<String> {
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    Ok(raw_string.to_str()?.to_owned())
}

struct App {
    _entry: ash::Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    surface_fn: ash::extensions::khr::Surface,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

struct ValidationInfo {
    pub is_enabled: bool,
    pub required_validation_layers: [&'static str; 1],
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        _ => "[UNKNOWN]",
    };

    let message = CStr::from_ptr((*p_callback_data).p_message);
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::debug!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::warn!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::warn!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::info!("{} {:?}", message_type, message),
        _ => log::trace!("{} {:?}", message_type, message),
    }

    vk::FALSE
}

fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback))
        .build()
}

impl App {
    fn new() -> Result<(EventLoop<()>, Window, Self)> {
        let entry = ash::Entry::new()?;

        let event_loop = EventLoop::new();
        let window = Self::init_window(&event_loop)?;
        let instance = Self::create_instance(&entry, &window)?;

        let (debug_utils_loader, debug_messenger) = Self::setup_debug_utils(&entry, &instance)?;

        let surface = unsafe { ash_window::create_surface(&entry, &instance, &window, None)? };
        let surface_fn = ash::extensions::khr::Surface::new(&entry, &instance);
        log::debug!("created surface: {:?}", surface);

        Ok((
            event_loop,
            window,
            Self {
                _entry: entry,
                instance,
                surface,
                surface_fn,
                debug_utils_loader,
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

    fn check_validation_error_support(entry: &ash::Entry) -> Result<()> {
        let layer_properties = entry.enumerate_instance_layer_properties()?;

        if layer_properties.is_empty() {
            return Err(Error::msg("no available layers found"));
        }

        for required_layer_name in VALIDATION.required_validation_layers.iter() {
            let mut is_layer_found = false;

            for layer_property in layer_properties.iter() {
                let test_layer_name = from_vk_string(&layer_property.layer_name)?;
                if *required_layer_name == test_layer_name {
                    is_layer_found = true;
                    break;
                }
            }

            if is_layer_found == false {
                return Err(Error::msg(format!(
                    "required layer `{}` was not found",
                    required_layer_name
                )));
            }
        }

        Ok(())
    }

    fn create_instance(entry: &ash::Entry, window: &Window) -> Result<ash::Instance> {
        if VALIDATION.is_enabled {
            Self::check_validation_error_support(entry)?;
        }

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
        let mut debug_utils_create_info = populate_debug_messenger_create_info();

        if VALIDATION.is_enabled {
            instance_info = instance_info.push_next(&mut debug_utils_create_info);
        }

        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        log::debug!("created instance");

        Ok(instance)
    }

    fn setup_debug_utils(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<(ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT)> {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        if VALIDATION.is_enabled {
            let messenger_create_info = populate_debug_messenger_create_info();

            let messenger = unsafe { debug_utils_loader.create_debug_utils_messenger(&messenger_create_info, None)? };

            log::debug!("created debug utils messenger: {:?}", messenger);

            Ok((debug_utils_loader, messenger))
        } else {
            Ok((debug_utils_loader, vk::DebugUtilsMessengerEXT::null()))
        }
    }

    fn draw_frame(&mut self) {
        // drawing here
    }

    fn run(mut self, event_loop: EventLoop<()>, window: Window) -> ! {
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
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
                log::debug!("dropped debug utils messenger");
            }

            self.surface_fn.destroy_surface(self.surface, None);
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
    }
}
