#![windows_subsystem = "windows"]

mod command_buffer;
mod frame;
mod framebuffer;
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

use crate::command_buffer::CommandPool;
use crate::frame::{Frame, SimpleFrameLogic};
use crate::instance::Instance;
use crate::logical_device::LogicalDevice;
use crate::pipeline::PipelineCache;
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
    pipeline_cache: PipelineCache,
    command_pool: CommandPool,

    frame: Frame<SimpleFrameLogic>,

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
        let swapchain = Swapchain::new(instance.get(), &surface, &logical_device, &window)?;
        let pipeline_cache = PipelineCache::new(&logical_device)?;
        let command_pool = CommandPool::new(&logical_device)?;

        let frame_logic = SimpleFrameLogic::new(&logical_device, &pipeline_cache, &command_pool, &swapchain)?;
        let frame = Frame::new(&logical_device, frame_logic)?;

        Ok((
            event_loop,
            window,
            Self {
                logical_device,
                validation,
                surface,
                instance,
                swapchain,
                pipeline_cache,
                command_pool,
                frame,
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

    fn draw_frame(&mut self) -> Result<()> {
        self.frame.draw(&self.logical_device, &self.swapchain)
    }

    fn run(mut self, event_loop: EventLoop<()>, _window: Window) -> ! {
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                if let Err(e) = self.draw_frame() {
                    log::error!("draw_frame error: {:?}", e);
                }
            }
            Event::LoopDestroyed => {
                if let Err(e) = self.logical_device.wait_idle() {
                    log::error!("failed to wait device idle: {:?}", e);
                }
            }
            _ => {}
        })
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.frame.destroy(&self.logical_device, &self.command_pool);
            self.command_pool.destroy(&self.logical_device);
            self.pipeline_cache.destroy(&self.logical_device);
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
