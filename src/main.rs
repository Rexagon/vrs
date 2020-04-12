#![windows_subsystem = "windows"]

extern crate nalgebra_glm as glm;

mod rendering;

use std::sync::Arc;

use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{Instance, PhysicalDevice, QueueFamily};
use vulkano::swapchain::Surface;
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::rendering::*;

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
            khr_maintenance1: true,
            ..DeviceExtensions::none()
        },
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("Failed to create device");

    let queue = queues.next().unwrap();

    let mut camera = Camera::new(surface.clone());
    camera.set_view(glm::look_at_rh(
        &glm::Vec3::new(2.0, 2.0, 2.0),
        &glm::Vec3::new(0.0, 0.0, 0.0),
        &glm::Vec3::new(0.0, 1.0, 0.0),
    ));

    let mut frame_system = FrameSystem::new(surface.clone(), queue.clone());
    let mut mesh_draw_system = MeshDrawSystem::new(queue.clone(), frame_system.deferred_subpass(), &camera);

    let mesh = SimpleMesh::new(queue.clone(), "./models/cube.glb");
    let mesh_state = MeshState {
        transform: glm::translation(&glm::Vec3::new(0.0, 0.0, 0.0)),
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
            camera.update_projection();
            mesh_draw_system.update_view(&camera);
        }
        Event::RedrawEventsCleared => {
            let mut frame = match frame_system.frame() {
                Some(frame) => frame,
                None => return,
            };

            while let Some(pass) = frame.next_pass() {
                match pass {
                    Pass::Draw(mut draw_pass) => {
                        draw_pass.execute(mesh_draw_system.draw(draw_pass.dynamic_state(), &mesh, &mesh_state))
                    }
                    Pass::Lighting(mut lighting_pass) => {
                        lighting_pass.ambient(0.1, [1.0, 1.0, 1.0]);
                        lighting_pass.directional(0.5, [1.0, 0.1, 0.1], [-1.0, 0.0, 0.0]);
                        lighting_pass.directional(0.5, [0.1, 1.1, 0.1], [0.0, -1.0, 0.0]);
                        lighting_pass.directional(0.5, [0.1, 0.1, 1.0], [0.0, 0.0, -1.0]);
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

#[derive(Clone)]
pub struct Camera {
    pub view: glm::Mat4,
    pub projection: glm::Mat4,

    surface: Arc<Surface<Window>>,
}

impl Camera {
    pub fn new(surface: Arc<Surface<Window>>) -> Self {
        let mut camera = Self {
            view: glm::identity(),
            projection: glm::identity(),
            surface,
        };
        camera.update_projection();

        camera
    }

    #[inline]
    pub fn set_view(&mut self, view: glm::Mat4) {
        self.view = view;
    }

    #[inline]
    pub fn update_projection(&mut self) {
        let size = self.surface.window().inner_size();
        let aspect = size.width as f32 / size.height as f32;

        let mut projection = glm::infinite_perspective_rh_zo(aspect, f32::to_radians(75.0), 0.01);
        projection.m22 *= -1.0;

        self.projection = projection;
    }
}

impl ViewDataSource for Camera {
    fn view(&self) -> glm::Mat4 {
        self.view.clone()
    }

    fn projection(&self) -> glm::Mat4 {
        self.projection.clone()
    }
}
