#![windows_subsystem = "windows"]

mod rendering;

use vulkano::{
    device::{Device, DeviceExtensions, Features},
    instance::{Instance, PhysicalDevice, QueueFamily},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::rendering::{FrameSystem, Pass, WorldState};
use nalgebra::Matrix4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None)?
    };

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_min_inner_size(LogicalSize::new(800, 600))
        .with_inner_size(LogicalSize::new(1280, 768))
        .with_title("vrs")
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("No device available");

    let queue_family = physical
        .queue_families()
        .filter(|&family| family.supports_graphics() && surface.is_supported(family).unwrap_or(false))
        .fold(None, |result: Option<QueueFamily>, family| match result {
            Some(result) if family.queues_count() > result.queues_count() => Some(family),
            Some(_) => result,
            _ => Some(family),
        })
        .expect("Failed to find a graphical queue family");

    let (_device, mut queues) = Device::new(
        physical,
        &Features::none(),
        &DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            khr_swapchain: true,
            ..DeviceExtensions::none()
        },
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("Failed to create device");

    let queue = queues.next().unwrap();

    let mut frame_system = FrameSystem::new(surface, queue);

    events_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            frame_system.invalidate_swapchain();
        }
        Event::RedrawEventsCleared => {
            let world_state = WorldState {
                world_matrix: Matrix4::identity(),
            };

            let mut frame = match frame_system.frame(&world_state) {
                Some(frame) => frame,
                None => return,
            };

            while let Some(pass) = frame.next_pass() {
                match pass {
                    Pass::Draw(_draw_pass) => {
                        // TODO
                    }
                    Pass::Lighting(_lighting_pass) => {
                        // TODO
                    }
                }
            }
        }
        _ => (),
    });
}
