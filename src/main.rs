#![windows_subsystem = "windows"]

#[macro_use]
extern crate memoffset;

mod camera;
mod input;
mod rendering;
mod scene;

extern crate nalgebra_glm as glm;

use std::time::Instant;

use anyhow::Result;
use rendering::*;
use winit::dpi::LogicalSize;
use winit::event::VirtualKeyCode;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::monitor::MonitorHandle;
use winit::window::Window;

use crate::camera::{Camera, FirstPersonController};
use crate::input::{InputState, InputStateHandler};
use crate::rendering::frame::SimpleFrameLogic;
use crate::scene::Scene;

const IS_VALIDATION_ENABLED: bool = true;

struct App {
    primary_monitor: MonitorHandle,

    device: Device,
    surface: Surface,
    validation: Validation,
    instance: Instance,
    swapchain: Swapchain,
    pipeline_cache: PipelineCache,
    command_pool: CommandPool,

    scene: Scene,
    frame: Frame<SimpleFrameLogic>,

    now: Instant,
    input_state: InputState,
    input_state_handler: InputStateHandler,
    camera_controller: FirstPersonController,

    is_fullscreen: bool,
    is_running: bool,

    _entry: ash::Entry,
}

impl App {
    fn new() -> Result<(EventLoop<()>, Window, Self)> {
        let entry = ash::Entry::new()?;

        let event_loop = EventLoop::new();
        let window = Self::init_window(&event_loop)?;

        let primary_monitor = event_loop.primary_monitor();

        let instance = Instance::new(&entry, &window, IS_VALIDATION_ENABLED)?;
        let validation = Validation::new(&entry, &instance, IS_VALIDATION_ENABLED)?;
        let surface = Surface::new(&entry, &instance, &window)?;
        let device = Device::new(&instance, &surface, IS_VALIDATION_ENABLED)?;
        let swapchain = Swapchain::new(&instance, &surface, &device, &window)?;
        let pipeline_cache = PipelineCache::new(&device)?;
        let command_pool = CommandPool::new(&device)?;

        let scene = Scene::new(&device, &command_pool, "./models/monkey.glb")?;

        let frame_logic = SimpleFrameLogic::new(&instance, &device, &pipeline_cache, &command_pool, &swapchain)?;
        let mut frame = Frame::new(&device, &swapchain, frame_logic)?;
        frame.logic_mut().update_meshes(scene.meshes());
        frame
            .logic_mut()
            .recreate_command_buffers(&device, &command_pool, &swapchain)?;

        let now = Instant::now();
        let input_state = InputState::new();
        let input_state_handler = InputStateHandler::new();
        let camera = Camera::new(window.inner_size());
        let camera_controller = FirstPersonController::new(camera, glm::vec3(0.0, 0.0, 1.0));

        Ok((
            event_loop,
            window,
            Self {
                primary_monitor,
                device,
                validation,
                surface,
                instance,
                swapchain,
                pipeline_cache,
                command_pool,
                scene,
                frame,
                now,
                input_state,
                input_state_handler,
                camera_controller,
                is_fullscreen: false,
                is_running: true,
                _entry: entry,
            },
        ))
    }

    fn init_window(event_loop: &EventLoop<()>) -> Result<winit::window::Window> {
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024, 768))
            .build(event_loop)?;

        Ok(window)
    }

    fn draw_frame(&mut self, window: &Window) -> Result<()> {
        let window_size = window.inner_size();

        let then = Instant::now();
        let dt = (then - self.now).as_secs_f32();
        self.now = then;

        self.input_state_handler.flush();
        self.input_state.update(&self.input_state_handler);
        self.camera_controller.handle_movement(window, &self.input_state, dt);

        if self.input_state.keyboard().was_pressed(VirtualKeyCode::Escape) {
            self.is_running = false;
            return Ok(());
        }

        if window_size.width == 0 || window_size.height == 0 {
            return Ok(());
        }

        if self.input_state.keyboard().was_pressed(VirtualKeyCode::F) {
            if self.is_fullscreen {
                window.set_fullscreen(None);
            } else {
                //window.set_inner_size(self.primary_monitor.size());
                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                    self.primary_monitor.clone(),
                )));
                window.set_always_on_top(false);
            }

            self.is_fullscreen = !self.is_fullscreen;
        }

        let current_frame = self.frame.current_frame();
        let camera = self.camera_controller.camera();
        self.frame
            .logic_mut()
            .pipeline_layout_mut()
            .uniform_buffers_mut()
            .update_world_data(&self.device, current_frame, camera.view(), camera.projection())?;

        let was_resized = self.frame.draw(&self.device, &self.swapchain)?;
        if was_resized {
            self.device.wait_idle()?;
            unsafe {
                self.swapchain.destroy(&self.device);
            }
            self.swapchain = Swapchain::new(&self.instance, &self.surface, &self.device, window)?;
            self.frame
                .recreate_logic(&self.device, &self.command_pool, &self.swapchain)?;
        }

        Ok(())
    }

    fn run(mut self, event_loop: EventLoop<()>, window: Window) -> ! {
        event_loop.run(move |event, _, control_flow| {
            if !self.is_running {
                if let Err(e) = self.device.wait_idle() {
                    log::error!("failed to wait device idle: {:?}", e);
                }
                *control_flow = ControlFlow::Exit;
                return;
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    self.camera_controller.camera_mut().update_projection(size);
                }
                Event::WindowEvent { ref event, .. } => {
                    self.input_state_handler.handle_window_event(event);
                }
                Event::MainEventsCleared => window.request_redraw(),
                Event::RedrawRequested(_) => {
                    if let Err(e) = self.draw_frame(&window) {
                        log::error!("draw_frame error: {:?}", e);
                    }
                }
                Event::LoopDestroyed => {
                    if let Err(e) = self.device.wait_idle() {
                        log::error!("failed to wait device idle: {:?}", e);
                    }
                }
                _ => {}
            }
        })
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.frame.destroy(&self.device, &self.command_pool);
            self.scene.destroy(&self.device);
            self.command_pool.destroy(&self.device);
            self.pipeline_cache.destroy(&self.device);
            self.swapchain.destroy(&self.device);
            self.device.destroy();
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
