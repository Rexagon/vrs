use std::sync::Arc;

use nalgebra::Matrix4;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageAccess, ImageViewAccess, SwapchainImage};
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{
    AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync::{GpuFuture, SharingMode};
use winit::window::Window;

pub struct FrameSystem {
    surface: Arc<Surface<Window>>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<Window>>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,

    dynamic_state: DynamicState,

    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    attachments: Attachments,
    should_recreate_swapchain: bool,
}

impl FrameSystem {
    pub fn new(surface: Arc<Surface<Window>>, queue: Arc<Queue>, format: Format) -> Self {
        let dimensions = surface.window().inner_size().into();

        let (swapchain, swapchain_images) = {
            let surface_capabilities = surface
                .capabilities(queue.device().physical_device())
                .expect("Failed to get surface capabilities");

            let usage = surface_capabilities.supported_usage_flags;
            let alpha = surface_capabilities.supported_composite_alpha.iter().next().unwrap();
            let format = surface_capabilities.supported_formats[0].0;

            Swapchain::new(
                queue.device().clone(),
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

        let attachments = Self::create_attachments(queue.device().clone(), dimensions);

        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(queue.device().clone(),
                attachments: {
                    final_color: {
                        load: Clear,
                        store: Store,
                        format: format,
                        samples: 1,
                    },
                    diffuse: {
                        load: Clear,
                        store: DontCare,
                        format: ImageViewAccess::format(&attachments.diffuse),
                        samples: 1,
                    },
                    normals: {
                        load: Clear,
                        store: DontCare,
                        format: ImageViewAccess::format(&attachments.normals),
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: ImageViewAccess::format(&attachments.depth),
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [diffuse, normals],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [final_color],
                        depth_stencil: {},
                        input: [diffuse, normals, depth]
                    }
                ]
            )
            .unwrap(),
        );

        let mut dynamic_state = DynamicState::none();

        let framebuffers = Self::create_framebuffers(
            dimensions,
            swapchain_images,
            &attachments,
            render_pass.clone(),
            &mut dynamic_state,
        );

        let _lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();
        // TODO: add lighting system

        Self {
            surface,
            queue,
            swapchain,
            framebuffers,
            dynamic_state,
            render_pass: render_pass as Arc<_>,
            attachments,
            should_recreate_swapchain: false,
        }
    }

    #[inline]
    pub fn deferred_subpass(&self) -> Subpass<Arc<dyn RenderPassAbstract + Send + Sync>> {
        Subpass::from(self.render_pass.clone(), 0).unwrap()
    }

    #[inline]
    pub fn invalidate_swapchain(&mut self) {
        self.should_recreate_swapchain = true;
    }

    pub fn frame<F, I>(&mut self, last_future: F, world: Matrix4<f32>)
    where
        F: GpuFuture + 'static,
    {
        if self.should_recreate_swapchain {
            let dimensions = self.surface.window().inner_size().into();
            let (swapchain, swapchain_images) = match self.swapchain.recreate_with_dimensions(dimensions) {
                Ok(result) => result,
                Err(SwapchainCreationError::UnsupportedDimensions) => return, // TODO: return None
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = swapchain;
            self.framebuffers = Self::create_framebuffers(
                dimensions,
                swapchain_images,
                &self.attachments,
                self.render_pass.clone(),
                &mut self.dynamic_state,
            );

            self.attachments = Self::create_attachments(self.queue.device().clone(), dimensions);

            self.should_recreate_swapchain = false;
        }

        let (swapchain_image_index, suboptimal, acquire_future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(result) => result,
                Err(AcquireError::OutOfDate) => {
                    self.should_recreate_swapchain = true;
                    return; // TODO: return None
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.should_recreate_swapchain = true;
        }

        let command_buffer = Some(
            AutoCommandBufferBuilder::primary_one_time_submit(self.queue.device().clone(), self.queue.family())
                .unwrap()
                .begin_render_pass(
                    self.framebuffers[swapchain_image_index].clone(),
                    true,
                    vec![
                        [0.0, 0.0, 0.0, 0.0].into(),
                        [0.0, 0.0, 0.0, 0.0].into(),
                        [0.0, 0.0, 0.0, 0.0].into(),
                        1.0f32.into(),
                    ],
                )
                .unwrap(),
        );
    }

    #[inline]
    fn create_attachments(device: Arc<Device>, dimensions: [u32; 2]) -> Attachments {
        let diffuse =
            AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::A2B10G10R10UnormPack32)
                .unwrap();

        let normals =
            AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::A2B10G10R10UnormPack32)
                .unwrap();

        let depth = AttachmentImage::transient_input_attachment(device, dimensions, Format::D32Sfloat).unwrap();

        Attachments {
            diffuse,
            normals,
            depth,
        }
    }

    #[inline]
    fn create_framebuffers(
        dimensions: [u32; 2],
        swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
        attachments: &Attachments,
        render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
        dynamic_state: &mut DynamicState,
    ) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        dynamic_state.viewports = Some(vec![viewport]);

        swapchain_images
            .into_iter()
            .map(move |image| {
                Arc::new(
                    Framebuffer::start(render_pass.clone())
                        .add(image.clone())
                        .unwrap()
                        .add(attachments.diffuse.clone())
                        .unwrap()
                        .add(attachments.normals.clone())
                        .unwrap()
                        .add(attachments.depth.clone())
                        .unwrap()
                        .build()
                        .unwrap(),
                ) as Arc<_>
            })
            .collect()
    }
}

struct Attachments {
    diffuse: Arc<AttachmentImage>,
    normals: Arc<AttachmentImage>,
    depth: Arc<AttachmentImage>,
}
