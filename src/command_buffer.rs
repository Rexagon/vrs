use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::framebuffer::Framebuffer;
use crate::logical_device::{LogicalDevice, Queues};
use crate::pipeline::{DefaultPipeline, SimpleRenderPass};
use crate::surface::Surface;
use crate::swapchain::Swapchain;

pub struct CommandPool {
    command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(logical_device: &LogicalDevice) -> Result<Self> {
        let command_pool = create_command_pool(logical_device)?;
        log::debug!("created command pool {:?}", command_pool);

        Ok(Self { command_pool })
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.command_pool
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.handle().destroy_command_pool(self.command_pool, None);
        log::debug!("dropped command pool {:?}", self.command_pool);
    }
}

fn create_command_pool(logical_device: &LogicalDevice) -> Result<vk::CommandPool> {
    let command_pool_create_info =
        vk::CommandPoolCreateInfo::builder().queue_family_index(logical_device.queues().graphics_queue_family);

    let command_pool = unsafe {
        logical_device
            .handle()
            .create_command_pool(&command_pool_create_info, None)?
    };

    Ok(command_pool)
}

pub fn create_command_buffers(
    logical_device: &LogicalDevice,
    command_pool: &CommandPool,
    graphics_pipeline: &DefaultPipeline,
    framebuffers: &[Framebuffer],
    render_pass: &SimpleRenderPass,
    swapchain: &Swapchain,
) -> Result<Vec<vk::CommandBuffer>> {
    let device = logical_device.handle();

    let command_buffer_create_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool.handle())
        .command_buffer_count(framebuffers.len() as u32)
        .level(vk::CommandBufferLevel::PRIMARY);

    let command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_create_info)? };

    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        let command_buffer_begin_info =
            vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

        unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info)? }

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle())
            .framebuffer(framebuffers[i].handle())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent(),
            })
            .clear_values(&clear_values);

        unsafe {
            device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline.handle(),
            );
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;
        }
    }

    Ok(command_buffers)
}

#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct CurrentFrame(usize);

impl CurrentFrame {
    pub fn new() -> Self {
        Default::default()
    }
}

pub struct FrameSyncObjects {
    max_frames_in_flight: usize,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    inflight_fences: Vec<vk::Fence>,
}

impl FrameSyncObjects {
    pub fn new(logical_device: &LogicalDevice, max_frames_in_flight: usize) -> Result<Self> {
        let device = logical_device.handle();

        let mut result = Self {
            max_frames_in_flight,
            image_available_semaphores: Vec::with_capacity(max_frames_in_flight),
            render_finished_semaphores: Vec::with_capacity(max_frames_in_flight),
            inflight_fences: Vec::with_capacity(max_frames_in_flight),
        };

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();

        let fence_create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        for _ in 0..max_frames_in_flight {
            unsafe {
                let image_available_semaphore = device.create_semaphore(&semaphore_create_info, None)?;
                log::debug!("created semaphore {:?}", image_available_semaphore);
                result.image_available_semaphores.push(image_available_semaphore);

                let render_finished_semaphore = device.create_semaphore(&semaphore_create_info, None)?;
                log::debug!("created semaphore {:?}", render_finished_semaphore);
                result.render_finished_semaphores.push(render_finished_semaphore);

                let inflight_fence = device.create_fence(&fence_create_info, None)?;
                log::debug!("created fence {:?}", inflight_fence);
                result.inflight_fences.push(inflight_fence);
            }
        }

        Ok(result)
    }

    pub fn wait_for_fence(&self, logical_device: &LogicalDevice, frame: CurrentFrame) -> Result<()> {
        let fences = [self.inflight_fences[frame.0]];

        unsafe { logical_device.handle().wait_for_fences(&fences, true, std::u64::MAX)? }

        Ok(())
    }

    pub fn reset_fences(&self, logical_device: &LogicalDevice, frame: CurrentFrame) -> Result<()> {
        let fences = [self.inflight_fences[frame.0]];

        unsafe { logical_device.handle().reset_fences(&fences)? };

        Ok(())
    }

    #[inline]
    pub fn image_available_semaphore(&self, frame: CurrentFrame) -> vk::Semaphore {
        self.image_available_semaphores[frame.0]
    }

    #[inline]
    pub fn render_finished_semaphore(&self, frame: CurrentFrame) -> vk::Semaphore {
        self.render_finished_semaphores[frame.0]
    }

    #[inline]
    pub fn inflight_fence(&self, frame: CurrentFrame) -> vk::Fence {
        self.inflight_fences[frame.0]
    }

    #[inline]
    pub fn next_frame(&self, frame: CurrentFrame) -> CurrentFrame {
        CurrentFrame((frame.0 + 1) % self.max_frames_in_flight)
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        let device = logical_device.handle();

        for i in 0..self.max_frames_in_flight {
            device.destroy_semaphore(self.image_available_semaphores[i], None);
            log::debug!("dropped semaphore {:?}", self.image_available_semaphores[i]);

            device.destroy_semaphore(self.render_finished_semaphores[i], None);
            log::debug!("dropped semaphore {:?}", self.render_finished_semaphores[i]);

            device.destroy_fence(self.inflight_fences[i], None);
            log::debug!("dropped fence {:?}", self.inflight_fences[i]);
        }
    }
}
