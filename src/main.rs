#![windows_subsystem = "windows"]

mod logical_device;
mod surface;
mod utils;
mod validation;

extern crate nalgebra_glm as glm;

use std::ffi::{CStr, CString};

use anyhow::Result;
use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::logical_device::LogicalDevice;
use crate::surface::Surface;
use crate::validation::Validation;

const APPLICATION_NAME: &str = "vrs";
const ENGINE_TITLE: &str = "ash";
const APPLICATION_VERSION: u32 = vk::make_version(1, 0, 0);
const ENGINE_VERSION: u32 = vk::make_version(1, 0, 0);
const IS_VALIDATION_ENABLED: bool = true;

struct App {
    logical_device: LogicalDevice,
    surface: Surface,
    validation: Validation,

    instance: ash::Instance,

    _entry: ash::Entry,
}

impl App {
    fn new() -> Result<(EventLoop<()>, Window, Self)> {
        let entry = ash::Entry::new()?;

        let event_loop = EventLoop::new();
        let window = Self::init_window(&event_loop)?;

        let instance = Self::create_instance(&entry, &window, IS_VALIDATION_ENABLED)?;
        let validation = Validation::new(&entry, &instance, IS_VALIDATION_ENABLED)?;
        let surface = Surface::new(&entry, &instance, &window)?;

        let logical_device = LogicalDevice::new(&instance, &surface, IS_VALIDATION_ENABLED)?;

        Ok((
            event_loop,
            window,
            Self {
                logical_device,
                validation,
                surface,
                instance,
                _entry: entry,
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

    fn create_instance(entry: &ash::Entry, window: &Window, is_validation_enabled: bool) -> Result<ash::Instance> {
        if is_validation_enabled {
            validation::check_supported(entry)?;
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
        let required_layers = if is_validation_enabled {
            validation::required_layers()
        } else {
            &[]
        };

        required_layers.iter().for_each(|layer| {
            log::debug!("required layer: {:?}", layer);
        });

        let required_layers: Vec<*const i8> = required_layers
            .into_iter()
            .map(|item| item.as_ptr())
            .collect::<Vec<_>>();

        //
        let mut instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&required_extensions)
            .enabled_layer_names(&required_layers);

        //
        let mut debug_utils_creation_info = validation::debug_messenger_create_info();
        if is_validation_enabled {
            instance_info = instance_info.push_next(&mut debug_utils_creation_info);
        }

        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        log::debug!("created instance");

        Ok(instance)
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
            self.logical_device.destroy();
            self.surface.destroy();
            self.validation.destroy();

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
