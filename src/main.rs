#![windows_subsystem = "windows"]

extern crate nalgebra_glm as glm;

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

use crate::rendering::{FrameSystem, MeshDrawSystem, MeshState, Pass, WorldState};

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

    let mut frame_system = FrameSystem::new(surface.clone(), queue.clone());
    let mut mesh_draw_system = MeshDrawSystem::new(queue.clone(), frame_system.deferred_subpass());

    let mesh = mesh_draw_system.create_simple_mesh();

    let create_world_state = {
        let surface = surface.clone();
        move || WorldState {
            view: glm::look_at_rh(
                &glm::Vec3::new(0.0, 3.0, -10.0),
                &glm::Vec3::identity(),
                &glm::Vec3::new(0.0, -1.0, 0.0),
            ),
            projection: glm::infinite_perspective_rh_zo(surface.window().aspect(), 0.5, 0.01),
        }
    };

    mesh_draw_system.set_world_state(create_world_state());
    let mesh_state = MeshState {
        transform: glm::Mat4::identity(),
    };

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
            mesh_draw_system.set_world_state(create_world_state());
        }
        Event::RedrawEventsCleared => {
            let mut frame = match frame_system.frame() {
                Some(frame) => frame,
                None => return,
            };

            while let Some(pass) = frame.next_pass() {
                match pass {
                    Pass::Draw(mut draw_pass) => draw_pass.execute(mesh_draw_system.draw(
                        draw_pass.dynamic_state(),
                        mesh.0.clone(),
                        mesh.1.clone(),
                        &mesh_state,
                    )),
                    Pass::Lighting(mut lighting_pass) => {
                        lighting_pass.ambient([0.2, 0.2, 0.2]);
                        lighting_pass.directional([1.0, 1.0, 1.0], [-0.5, 0.4, -0.5]);
                        lighting_pass.directional([0.1, 0.1, 0.1], [0.5, 0.5, 0.5]);
                    }
                    Pass::Compose(mut composing_pass) => {
                        composing_pass.compose();
                    }
                }
            }
        }
        _ => (),
    });
}

trait WindowExt {
    fn aspect(&self) -> f32;
}

impl WindowExt for winit::window::Window {
    fn aspect(&self) -> f32 {
        let size = self.inner_size();
        size.width as f32 / size.height as f32
    }
}
