#![windows_subsystem = "windows"]

mod instance;
mod logical_device;
mod pipeline;
mod shader;
mod surface;
mod swapchain;
mod utils;
mod validation;

extern crate nalgebra_glm as glm;

use anyhow::Result;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::instance::Instance;
use crate::logical_device::LogicalDevice;
use crate::pipeline::{DefaultPipeline, SimpleRenderPass};
use crate::surface::Surface;
use crate::swapchain::Swapchain;
use crate::validation::Validation;

const IS_VALIDATION_ENABLED: bool = true;

struct App {
    logical_device: LogicalDevice,
    surface: Surface,
    validation: Validation,
    instance: Instance,
    swapchain: Swapchain,
    simple_render_pass: SimpleRenderPass,
    pipeline: DefaultPipeline,

    _entry: ash::Entry,
}

impl App {
    fn new() -> Result<(EventLoop<()>, Window, Self)> {
        let entry = ash::Entry::new()?;

        let event_loop = EventLoop::new();
        let window = Self::init_window(&event_loop)?;

        let instance = Instance::new(&entry, &window, IS_VALIDATION_ENABLED)?;
        let validation = Validation::new(&entry, instance.get(), IS_VALIDATION_ENABLED)?;
        let surface = Surface::new(&entry, instance.get(), &window)?;
        let logical_device = LogicalDevice::new(instance.get(), &surface, IS_VALIDATION_ENABLED)?;
        let swapchain = Swapchain::new(instance.get(), &surface, &logical_device)?;

        let simple_render_pass = SimpleRenderPass::new(&logical_device, swapchain.format())?;
        let pipeline = DefaultPipeline::new(&logical_device, swapchain.extent(), &simple_render_pass)?;

        Ok((
            event_loop,
            window,
            Self {
                logical_device,
                validation,
                surface,
                instance,
                swapchain,
                simple_render_pass,
                pipeline,
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
            self.pipeline.destroy(&self.logical_device);
            self.simple_render_pass.destroy(&self.logical_device);
            self.swapchain.destroy(&self.logical_device);
            self.logical_device.destroy();
            self.surface.destroy();
            self.validation.destroy();
            self.instance.destroy();
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
