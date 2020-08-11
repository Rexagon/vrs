#![windows_subsystem = "windows"]

mod command_buffer;
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
use ash::version::DeviceV1_0;
use ash::vk;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::command_buffer::{CommandPool, CurrentFrame, FrameSyncObjects};
use crate::framebuffer::Framebuffer;
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
    framebuffers: Vec<Framebuffer>,

    command_pool: CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    frame_sync_objects: FrameSyncObjects,
    current_frame: CurrentFrame,

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
        let framebuffers =
            swapchain
                .image_views()
                .iter()
                .try_fold(Vec::<Framebuffer>::new(), |mut framebuffers, &view| {
                    Framebuffer::new(&logical_device, simple_render_pass.handle(), view, swapchain.extent()).map(
                        |framebuffer| {
                            framebuffers.push(framebuffer);
                            framebuffers
                        },
                    )
                })?;

        let command_pool = CommandPool::new(&logical_device)?;

        let command_buffers = command_buffer::create_command_buffers(
            &logical_device,
            &command_pool,
            &pipeline,
            &framebuffers,
            &simple_render_pass,
            &swapchain,
        )?;

        let frame_sync_objects = FrameSyncObjects::new(&logical_device, 2)?;

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
                framebuffers,
                command_pool,
                command_buffers,
                frame_sync_objects,
                current_frame: Default::default(),
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
        let wait_semaphores = [self.frame_sync_objects.image_available_semaphore(self.current_frame)];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let wait_fence = self.frame_sync_objects.inflight_fence(self.current_frame);
        let signal_semaphores = [self.frame_sync_objects.render_finished_semaphore(self.current_frame)];

        self.frame_sync_objects
            .wait_for_fence(&self.logical_device, self.current_frame)?;

        let (image_index, _is_sub_optimal) = self.swapchain.acquire_next_image(wait_semaphores[0])?;

        let command_buffers = [self.command_buffers[image_index as usize]];

        self.frame_sync_objects
            .reset_fences(&self.logical_device, self.current_frame)?;

        let submit_infos = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)
            .build()];
        unsafe {
            self.logical_device.handle().queue_submit(
                self.logical_device.queues().graphics_queue,
                &submit_infos,
                wait_fence,
            )?;
        };

        self.swapchain
            .present_image(&self.logical_device, &signal_semaphores, image_index)?;

        self.current_frame = self.frame_sync_objects.next_frame(self.current_frame);

        Ok(())
    }

    fn run(mut self, event_loop: EventLoop<()>, window: Window) -> ! {
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
            self.frame_sync_objects.destroy(&self.logical_device);

            self.command_pool.destroy(&self.logical_device);

            self.framebuffers
                .iter()
                .for_each(|item| item.destroy(&self.logical_device));

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
