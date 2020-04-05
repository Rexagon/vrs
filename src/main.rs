#![windows_subsystem = "windows"]

use std::sync::Arc;

use vulkano::format::ClearValue;
use vulkano::image::ImageViewAccess;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    device::{Device, DeviceExtensions, Features},
    framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass},
    image::{AttachmentImage, SwapchainImage},
    instance::{Instance, PhysicalDevice, QueueFamily},
    pipeline::{viewport::Viewport, GraphicsPipeline},
    swapchain::{
        AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError,
    },
    sync::{FlushError, GpuFuture, SharingMode},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    window::WindowBuilder,
};

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

    log::debug!("{:?}", physical);

    for family in physical.queue_families() {
        log::debug!("{:?}", family);
        log::debug!("{:?} queue", family.queues_count());
        log::debug!("Supports graphics: {:?}", family.supports_graphics());
        log::debug!("Supports compute: {:?}", family.supports_compute());
    }

    let queue_family = physical
        .queue_families()
        .filter(|&family| family.supports_graphics() && surface.is_supported(family).unwrap_or(false))
        .fold(None, |result: Option<QueueFamily>, family| match result {
            Some(result) if family.queues_count() > result.queues_count() => Some(family),
            Some(_) => result,
            _ => Some(family),
        })
        .expect("Failed to find a graphical queue family");

    log::debug!("Selected family: {:?}", queue_family);

    let (device, mut queues) = Device::new(
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

    let (mut swapchain, images) = {
        let surface_capabilities = surface
            .capabilities(physical)
            .expect("Failed to get surface capabilities");

        let usage = surface_capabilities.supported_usage_flags;
        let alpha = surface_capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = surface_capabilities.supported_formats[0].0;

        let dimensions = surface.window().inner_size().into();

        Swapchain::new(
            device.clone(),
            surface.clone(),
            surface_capabilities.min_image_count,
            format,
            dimensions,
            1,
            usage,
            SharingMode::Exclusive,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .expect("Failed to create swapchain")
    };

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        [
            Vertex { position: [-0.5, -0.5] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.5, -0.25] },
        ]
        .iter()
        .cloned(),
    )
    .unwrap();

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                intermediary: {
                    load: Clear,
                    store: DontCare,
                    format: swapchain.format(),
                    samples: 4,
                },
                color: {
                    load: DontCare,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [intermediary],
                depth_stencil: {},
                resolve: [color]
            }
        )
        .unwrap(),
    );

    let vertex_shader = vertex_shader::Shader::load(device.clone()).expect("Failed to create shader module");
    let fragment_shader =
        fragment_shader::Shader::load(device.clone()).expect("Failed to create fragment shader module");

    let graphics_pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut dynamic_state = DynamicState::none();

    let mut framebuffers =
        window_size_dependent_setup(device.clone(), &images, render_pass.clone(), &mut dynamic_state);

    //

    let mut recreate_swapchain = false;

    let mut previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<dyn GpuFuture>);

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
            recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => {
            previous_frame_end.as_mut().unwrap().cleanup_finished();

            if recreate_swapchain {
                let dimensions = surface.window().inner_size().into();
                let (new_swapchain, new_images) = match swapchain.recreate_with_dimensions(dimensions) {
                    Ok(result) => result,
                    Err(SwapchainCreationError::UnsupportedDimensions) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

                swapchain = new_swapchain;
                framebuffers =
                    window_size_dependent_setup(device.clone(), &new_images, render_pass.clone(), &mut dynamic_state);
                recreate_swapchain = false;
            }

            let (image_num, suboptimal, acquire_future) =
                match vulkano::swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(result) => result,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            let clear_values = vec![[0.0, 0.0, 0.0, 0.0].into(), ClearValue::None];

            let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
                .unwrap()
                .draw(graphics_pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ())
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(Box::new(future) as Box<_>);
                }

                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>);
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>);
                }
            }
        }
        _ => (),
    });
}

fn window_size_dependent_setup(
    device: Arc<Device>,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions().width_height();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(move |image| {
            let intermediary =
                AttachmentImage::transient_multisampled(device.clone(), dimensions, 4, image.format()).unwrap();

            let framebuffer = Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(intermediary)
                    .unwrap()
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>;

            framebuffer
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default)]
struct Vertex {
    position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/triangle.vert",
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/triangle.frag",
    }
}
