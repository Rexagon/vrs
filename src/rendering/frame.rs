use super::prelude::*;
use super::{
    shader, utils, Buffer, CommandPool, Device, Framebuffer, Mesh, PipelineCache, ShaderModule, Swapchain, Vertex,
};

pub struct Frame<T> {
    logic: T,
    current_frame: usize,
    frame_sync_objects: FrameSyncObjects,
}

impl<T> Frame<T>
where
    T: FrameLogic,
{
    pub fn new(device: &Device, swapchain: &Swapchain, logic: T) -> Result<Self> {
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
    pub fn logic_mut(&mut self) -> &mut T {
        &mut self.logic
    }
}

pub trait FrameLogic {
    fn recreate_frame_buffers(&mut self, device: &Device, swapchain: &Swapchain) -> Result<()>;
    fn recreate_command_buffers(
        &mut self,
        device: &Device,
        command_pool: &CommandPool,
        swapchain: &Swapchain,
    ) -> Result<()>;
    fn command_buffer(&self, image_index: usize) -> vk::CommandBuffer;
    unsafe fn destroy(&self, device: &Device, command_pool: &CommandPool);
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

pub struct SimpleFrameLogic {
    simple_render_pass: SimpleRenderPass,
    pipeline_layout: SimplePipelineLayout,
    vertex_shader_module: ShaderModule,
    fragment_shader_module: ShaderModule,
    graphics_pipeline: vk::Pipeline,
    command_buffers: Vec<vk::CommandBuffer>,
    framebuffers: Vec<Framebuffer>,

    meshes: Vec<(vk::Buffer, vk::Buffer, u64, u32)>,
}

impl SimpleFrameLogic {
    pub fn new(
        device: &Device,
        pipeline_cache: &PipelineCache,
        command_pool: &CommandPool,
        swapchain: &Swapchain,
    ) -> Result<Self> {
        let simple_render_pass = SimpleRenderPass::new(device, swapchain.format())?;
        let pipeline_layout = SimplePipelineLayout::new(device, swapchain.image_views().len())?;
        let vertex_shader_module = ShaderModule::from_file(device, "shaders/spv/mesh.vert.spv")?;
        let fragment_shader_module = ShaderModule::from_file(device, "shaders/spv/mesh.frag.spv")?;

        let mut result = Self {
            simple_render_pass,
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
            graphics_pipeline: vk::Pipeline::null(),
            command_buffers: Vec::new(),
            framebuffers: Vec::new(),
            meshes: Vec::new(),
        };

        result.recreate_pipeline(device, pipeline_cache)?;
        result.recreate_frame_buffers(device, swapchain)?;
        result.recreate_command_buffers(device, command_pool, swapchain)?;

        Ok(result)
    }

    pub fn recreate_pipeline(&mut self, device: &Device, pipeline_cache: &PipelineCache) -> Result<()> {
        let main_function_name = shader::main_function_name();

        // shader stages
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfo::builder()
                .module(self.vertex_shader_module.handle())
                .name(main_function_name)
                .stage(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .module(self.fragment_shader_module.handle())
                .name(main_function_name)
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        // vertex input state
        let binding_descriptions = Vertex::get_binding_descriptions();
        let attribute_descriptions = Vertex::get_attribute_descriptions();

        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // viewports
        let viewports = [vk::Viewport::builder().build()];
        let scissors = [vk::Rect2D::builder().build()];

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        // rasterization state
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL);

        // multisample state
        let multisample_state_create_info =
            vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // depth state
        let stencil_state = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .depth_fail_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .compare_mask(0)
            .write_mask(0)
            .reference(0)
            .build();

        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(true)
            .stencil_test_enable(false)
            .front(stencil_state)
            .back(stencil_state);

        // color blend state
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::all())
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build()];

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachment_states);

        // dynamic state create info
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        // pipeline creation
        let graphics_pipeline_create_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .color_blend_state(&color_blend_state)
            .layout(self.pipeline_layout.handle())
            .render_pass(self.simple_render_pass.handle())
            .subpass(0)
            .dynamic_state(&dynamic_state_create_info)
            .base_pipeline_handle(self.graphics_pipeline)
            .base_pipeline_index(-1)
            .build()];

        let graphics_pipelines = unsafe {
            device
                .handle()
                .create_graphics_pipelines(pipeline_cache.handle(), &graphics_pipeline_create_infos, None)
                .map_err(|(_, e)| e)?
        };
        let graphics_pipeline = graphics_pipelines[0];
        log::debug!("create graphics pipeline {:?}", graphics_pipeline);

        if self.graphics_pipeline != vk::Pipeline::null() {
            unsafe { self.destroy_pipeline(device) };
        }

        self.graphics_pipeline = graphics_pipeline;

        Ok(())
    }

    pub fn update_meshes(&mut self, meshes: &[Mesh]) {
        self.meshes = meshes
            .iter()
            .map(|mesh| {
                (
                    mesh.vertex_buffer().handle(),
                    mesh.index_buffer().handle(),
                    0,
                    mesh.index_count(),
                )
            })
            .collect();
    }

    unsafe fn destroy_pipeline(&self, device: &Device) {
        device.handle().destroy_pipeline(self.graphics_pipeline, None);
        log::debug!("dropped pipeline {:?}", self.graphics_pipeline);
    }

    unsafe fn destroy_framebuffers(&self, device: &Device) {
        self.framebuffers.iter().for_each(|item| item.destroy(device));
    }

    unsafe fn free_command_buffers(&self, device: &Device, command_pool: &CommandPool) {
        device
            .handle()
            .free_command_buffers(command_pool.handle(), &self.command_buffers);
    }

    #[allow(unused)]
    #[inline]
    pub fn pipeline_layout(&self) -> &SimplePipelineLayout {
        &self.pipeline_layout
    }

    #[inline]
    pub fn pipeline_layout_mut(&mut self) -> &mut SimplePipelineLayout {
        &mut self.pipeline_layout
    }
}

impl FrameLogic for SimpleFrameLogic {
    fn recreate_frame_buffers(&mut self, device: &Device, swapchain: &Swapchain) -> Result<()> {
        // destroy framebuffers
        unsafe { self.destroy_framebuffers(device) };

        // create framebuffers
        self.framebuffers = swapchain.image_views().iter().try_fold(
            Vec::with_capacity(swapchain.image_views().len()),
            |mut framebuffers, &image_view| {
                Framebuffer::new(
                    device,
                    self.simple_render_pass.handle(),
                    &[image_view],
                    swapchain.extent(),
                )
                .map(|framebuffer| {
                    framebuffers.push(framebuffer);
                    framebuffers
                })
            },
        )?;

        // done
        Ok(())
    }

    fn recreate_command_buffers(
        &mut self,
        device: &Device,
        command_pool: &CommandPool,
        swapchain: &Swapchain,
    ) -> Result<()> {
        // free command buffers
        unsafe { self.free_command_buffers(device, command_pool) };

        let extent = swapchain.extent();

        // create command buffers
        let device = device.handle();

        let command_buffer_create_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.handle())
            .command_buffer_count(swapchain.image_count())
            .level(vk::CommandBufferLevel::PRIMARY);

        self.command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_create_info)? };

        let viewports = [utils::viewport_flipped(extent, 0.0, 1.0)];
        let scissors = [utils::rect_2d([0, 0], extent)];

        for (i, &command_buffer) in self.command_buffers.iter().enumerate() {
            let command_buffer_begin_info =
                vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

            unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info)? }

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.simple_render_pass.handle())
                .framebuffer(self.framebuffers[i].handle())
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                })
                .clear_values(&clear_values);

            unsafe {
                device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
                device.cmd_set_viewport(command_buffer, 0, &viewports);
                device.cmd_set_scissor(command_buffer, 0, &scissors);

                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline);

                for &(vertex_buffer, index_buffer, offset, index_count) in &self.meshes {
                    let vertex_buffers = [vertex_buffer];
                    let offsets = [offset];
                    let descriptor_sets = [self.pipeline_layout.uniform_buffers().descriptor_set(i)];

                    device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT16);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout.handle(),
                        0,
                        &descriptor_sets,
                        &[],
                    );
                    device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
                }

                device.cmd_end_render_pass(command_buffer);
                device.end_command_buffer(command_buffer)?;
            }
        }

        Ok(())
    }

    fn command_buffer(&self, image_index: usize) -> vk::CommandBuffer {
        self.command_buffers[image_index]
    }

    unsafe fn destroy(&self, device: &Device, command_pool: &CommandPool) {
        self.free_command_buffers(device, command_pool);
        self.destroy_framebuffers(device);
        self.destroy_pipeline(device);
        self.simple_render_pass.destroy(device);
        self.pipeline_layout.destroy(device);
        self.vertex_shader_module.destroy(device);
        self.fragment_shader_module.destroy(device);
    }
}

pub struct SimpleRenderPass {
    render_pass: vk::RenderPass,
}

impl SimpleRenderPass {
    fn new(device: &Device, surface_format: vk::Format) -> Result<Self> {
        // subpasses
        let color_attachment_ref = [vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_ref)
            .build()];

        // render pass
        let render_pass_attachments = [vk::AttachmentDescription::builder()
            .format(surface_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build()];

        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .subpasses(&subpasses)
            .attachments(&render_pass_attachments);

        let render_pass = unsafe { device.handle().create_render_pass(&render_pass_create_info, None)? };
        log::debug!("created render pass {:?}", render_pass);

        Ok(Self { render_pass })
    }

    unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_render_pass(self.render_pass, None);
        log::debug!("dropped render pass {:?}", self.render_pass);
    }

    #[inline]
    fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }
}

pub struct SimplePipelineLayout {
    pipeline_layout: vk::PipelineLayout,
    descriptor_pool: DescriptorPool,
    uniform_buffers: UniformBuffers,
}

impl SimplePipelineLayout {
    pub fn new(device: &Device, max_frames_in_flight: usize) -> Result<Self> {
        let descriptor_pool = DescriptorPool::new(device, max_frames_in_flight)?;
        let uniform_buffers = UniformBuffers::new(device, &descriptor_pool, max_frames_in_flight)?;

        let descriptor_set_layouts = [uniform_buffers.layout()];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);

        let pipeline_layout = unsafe {
            device
                .handle()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };
        log::debug!("created pipeline layout {:?}", pipeline_layout);

        Ok(Self {
            pipeline_layout,
            descriptor_pool,
            uniform_buffers,
        })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_pipeline_layout(self.pipeline_layout, None);
        log::debug!("dropped pipeline layout {:?}", self.pipeline_layout);

        self.uniform_buffers.destroy(device, &self.descriptor_pool);
        self.descriptor_pool.destroy(device);
    }

    #[inline]
    pub fn handle(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    #[inline]
    pub fn uniform_buffers(&self) -> &UniformBuffers {
        &self.uniform_buffers
    }

    #[inline]
    pub fn uniform_buffers_mut(&mut self) -> &mut UniformBuffers {
        &mut self.uniform_buffers
    }
}

pub struct UniformBuffers {
    descriptor_set_layout: vk::DescriptorSetLayout,
    world_data_buffers: Vec<Buffer>,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl UniformBuffers {
    pub fn new(device: &Device, descriptor_pool: &DescriptorPool, max_frames_in_flight: usize) -> Result<Self> {
        // create descriptor set layout
        let ubo_layout_bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];

        let ubo_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&ubo_layout_bindings);

        let descriptor_set_layout = unsafe {
            device
                .handle()
                .create_descriptor_set_layout(&ubo_layout_create_info, None)?
        };
        log::debug!("created descriptor set layout {:?}", descriptor_set_layout);

        // create buffers
        let buffer_size = (std::mem::size_of::<glm::Mat4>() * 2) as vk::DeviceSize;

        let world_data_buffers =
            (0..max_frames_in_flight).try_fold(Vec::with_capacity(max_frames_in_flight), |mut buffers, _| {
                Buffer::new(
                    device,
                    buffer_size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
                .map(|buffer| {
                    buffers.push(buffer);
                    buffers
                })
            })?;

        // create descriptor sets
        let layouts = std::iter::repeat(descriptor_set_layout)
            .take(max_frames_in_flight)
            .collect::<Vec<_>>();

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool.handle())
            .set_layouts(&layouts);
        let descriptor_sets = unsafe {
            device
                .handle()
                .allocate_descriptor_sets(&descriptor_set_allocate_info)?
        };

        // bind descriptor sets to buffers
        for (i, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let descriptor_buffer_info = [vk::DescriptorBufferInfo {
                buffer: world_data_buffers[i].handle(),
                offset: 0,
                range: world_data_buffers[i].size(),
            }];

            let descriptor_write_sets = [vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&descriptor_buffer_info)
                .build()];

            unsafe {
                device.handle().update_descriptor_sets(&descriptor_write_sets, &[]);
            }
        }

        // done
        Ok(Self {
            descriptor_set_layout,
            world_data_buffers,
            descriptor_sets,
        })
    }

    pub unsafe fn destroy(&self, device: &Device, descriptor_pool: &DescriptorPool) {
        self.world_data_buffers.iter().for_each(|buffer| buffer.destroy(device));

        let device = device.handle();

        device.free_descriptor_sets(descriptor_pool.handle(), &self.descriptor_sets);

        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        log::debug!("dropped descriptor set layout {:?}", self.descriptor_set_layout);
    }

    pub fn update_world_data(
        &mut self,
        device: &Device,
        current_frame: usize,
        view: &glm::Mat4,
        projection: &glm::Mat4,
    ) -> Result<()> {
        let buffer = &self.world_data_buffers[current_frame];

        unsafe {
            let data_ptr = buffer.map_memory(device)?;

            let mut buffer_data = [0f32; 16 * 2];
            buffer_data[..16].copy_from_slice(view.as_slice());
            buffer_data[16..].copy_from_slice(projection.as_slice());
            let buffer_data_slice = bytemuck::cast_slice(&buffer_data);

            data_ptr.copy_from_nonoverlapping(buffer_data_slice.as_ptr(), buffer_data_slice.len());

            buffer.unmap_memory(device);
        }

        Ok(())
    }

    #[inline]
    pub fn descriptor_set(&self, current_frame: usize) -> vk::DescriptorSet {
        self.descriptor_sets[current_frame]
    }

    #[inline]
    pub fn layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

pub struct DescriptorPool {
    descriptor_pool: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(device: &Device, size: usize) -> Result<Self> {
        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: size as u32,
        }];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(size as u32)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device
                .handle()
                .create_descriptor_pool(&descriptor_pool_create_info, None)?
        };
        log::debug!("created descriptor pool {:?}", descriptor_pool);

        Ok(Self { descriptor_pool })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        device.handle().destroy_descriptor_pool(self.descriptor_pool, None);
        log::debug!("dropped descriptor pool {:?}", self.descriptor_pool);
    }

    #[inline]
    pub fn handle(&self) -> vk::DescriptorPool {
        self.descriptor_pool
    }
}
