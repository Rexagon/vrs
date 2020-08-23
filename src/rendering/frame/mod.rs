mod deferred_render_pass;
mod frame_logic;
mod graphics_pipeline_layout;

use self::frame_logic::*;
use super::prelude::*;
use super::{CommandPool, Device, Instance, PipelineCache, Swapchain};

pub struct Frame {
    logic: FrameLogic,
    current_frame: usize,
    frame_sync_objects: FrameSyncObjects,
}

impl Frame {
    pub fn new(
        instance: &Instance,
        device: &Device,
        pipeline_cache: &PipelineCache,
        command_pool: &CommandPool,
        swapchain: &Swapchain,
    ) -> Result<Self> {
        let logic = FrameLogic::new(instance, device, pipeline_cache, command_pool, swapchain)?;

        let current_frame = 0;
        let frame_sync_objects = FrameSyncObjects::new(device, swapchain.image_views().len())?;

        Ok(Self {
            logic,
            current_frame,
            frame_sync_objects,
        })
    }

    pub unsafe fn destroy(&self, device: &Device, command_pool: &CommandPool) {
        self.logic.destroy(device, command_pool);
        self.frame_sync_objects.destroy(device);
    }

    pub fn draw(&mut self, device: &Device, swapchain: &Swapchain) -> Result<bool> {
        let wait_semaphores = [self.frame_sync_objects.image_available_semaphore(self.current_frame)];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let wait_fence = self.frame_sync_objects.inflight_fence(self.current_frame);
        let signal_semaphores = [self.frame_sync_objects.render_finished_semaphore(self.current_frame)];

        self.frame_sync_objects.wait_for_fence(device, self.current_frame)?;

        let image_index = match swapchain.acquire_next_image(wait_semaphores[0]) {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
            Err(e) => return Err(anyhow::Error::new(e)),
        };

        let command_buffers = [self.logic.command_buffer(image_index as usize)];

        self.frame_sync_objects.reset_fences(device, self.current_frame)?;

        let submit_infos = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)
            .build()];
        unsafe {
            device
                .handle()
                .queue_submit(device.queues().graphics_queue, &submit_infos, wait_fence)?;
        };

        let was_resized = swapchain.present_image(device, &signal_semaphores, image_index)?;

        self.current_frame = self.frame_sync_objects.next_frame(self.current_frame);

        Ok(was_resized)
    }

    pub fn recreate_logic(&mut self, device: &Device, command_pool: &CommandPool, swapchain: &Swapchain) -> Result<()> {
        self.logic.recreate_frame_buffers(device, swapchain)?;
        self.logic.recreate_command_buffers(device, command_pool, swapchain)
    }

    #[inline]
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    #[inline]
    pub fn logic_mut(&mut self) -> &mut FrameLogic {
        &mut self.logic
    }
}

pub struct FrameSyncObjects {
    max_frames_in_flight: usize,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    inflight_fences: Vec<vk::Fence>,
}

impl FrameSyncObjects {
    pub fn new(device: &Device, max_frames_in_flight: usize) -> Result<Self> {
        let device = device.handle();

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

    pub unsafe fn destroy(&self, device: &Device) {
        let device = device.handle();

        for i in 0..self.max_frames_in_flight {
            device.destroy_semaphore(self.image_available_semaphores[i], None);
            log::debug!("dropped semaphore {:?}", self.image_available_semaphores[i]);

            device.destroy_semaphore(self.render_finished_semaphores[i], None);
            log::debug!("dropped semaphore {:?}", self.render_finished_semaphores[i]);

            device.destroy_fence(self.inflight_fences[i], None);
            log::debug!("dropped fence {:?}", self.inflight_fences[i]);
        }
    }

    pub fn wait_for_fence(&self, device: &Device, frame: usize) -> Result<()> {
        let fences = [self.inflight_fences[frame]];
        unsafe { device.handle().wait_for_fences(&fences, true, std::u64::MAX)? }
        Ok(())
    }

    pub fn reset_fences(&self, device: &Device, frame: usize) -> Result<()> {
        let fences = [self.inflight_fences[frame]];
        unsafe { device.handle().reset_fences(&fences)? };
        Ok(())
    }

    #[inline]
    pub fn image_available_semaphore(&self, frame: usize) -> vk::Semaphore {
        self.image_available_semaphores[frame]
    }

    #[inline]
    pub fn render_finished_semaphore(&self, frame: usize) -> vk::Semaphore {
        self.render_finished_semaphores[frame]
    }

    #[inline]
    pub fn inflight_fence(&self, frame: usize) -> vk::Fence {
        self.inflight_fences[frame]
    }

    #[inline]
    pub fn next_frame(&self, frame: usize) -> usize {
        (frame + 1) % self.max_frames_in_flight
    }
}
