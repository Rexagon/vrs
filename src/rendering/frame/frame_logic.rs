use super::deferred_render_pass::DeferredRenderPass;
use super::graphics_pipeline_layout::GraphicsPipelineLayout;
use crate::rendering::prelude::*;
use crate::rendering::{shader, utils};
use crate::rendering::{
    CommandPool, Device, Framebuffer, Image, ImageView, Mesh, PipelineCache, ShaderModule, Swapchain, Vertex,
};

pub struct FrameLogic {
    device: Arc<Device>,
    command_pool: Arc<CommandPool>,

    deferred_render_pass: DeferredRenderPass,
    pipeline_layout: GraphicsPipelineLayout,
    vertex_shader_module: ShaderModule,
    fragment_shader_module: ShaderModule,
    graphics_pipeline: vk::Pipeline,
    command_buffers: Vec<vk::CommandBuffer>,
    framebuffers: Vec<(Framebuffer, Image, ImageView)>,
    depth_format: vk::Format,

    meshes: Vec<(vk::Buffer, vk::Buffer, u64, u32)>,
}

impl FrameLogic {
    pub fn new(
        device: Arc<Device>,
        pipeline_cache: &PipelineCache,
        command_pool: Arc<CommandPool>,
        swapchain: &Swapchain,
    ) -> Result<Self> {
        let depth_format = device.find_supported_format(
            &[
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;

        let deferred_render_pass = DeferredRenderPass::new(device.clone(), swapchain.format(), depth_format)?;
        let pipeline_layout = GraphicsPipelineLayout::new(device.clone(), swapchain.image_views().len())?;
        let vertex_shader_module = ShaderModule::from_file(device.clone(), "shaders/spv/mesh.vert.spv")?;
        let fragment_shader_module = ShaderModule::from_file(device.clone(), "shaders/spv/mesh.frag.spv")?;

        let main_function_name = shader::main_function_name();

        // shader stages
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfo::builder()
                .module(vertex_shader_module.handle())
                .name(main_function_name)
                .stage(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .module(fragment_shader_module.handle())
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
            .depth_bounds_test_enable(false)
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
            .layout(pipeline_layout.handle())
            .render_pass(deferred_render_pass.handle())
            .subpass(0)
            .dynamic_state(&dynamic_state_create_info)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(-1)
            .build()];

        let graphics_pipelines = unsafe {
            device
                .handle()
                .create_graphics_pipelines(pipeline_cache.handle(), &graphics_pipeline_create_infos, None)
                .map_err(|(_, e)| e)?
        };
        let graphics_pipeline = graphics_pipelines[0];

        let mut result = Self {
            device,
            command_pool,
            deferred_render_pass,
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
            graphics_pipeline,
            command_buffers: Vec::new(),
            framebuffers: Vec::new(),
            depth_format,
            meshes: Vec::new(),
        };

        result.recreate_frame_buffers(swapchain)?;
        result.recreate_command_buffers(swapchain)?;

        Ok(result)
    }

    unsafe fn destroy_framebuffers(&self) {
        self.framebuffers
            .iter()
            .for_each(|(framebuffer, depth_image, depth_image_view)| {
                depth_image_view.destroy();
                depth_image.destroy();
                framebuffer.destroy();
            });
    }

    unsafe fn free_command_buffers(&self) {
        self.device
            .handle()
            .free_command_buffers(self.command_pool.handle(), &self.command_buffers);
    }

    pub unsafe fn destroy(&self) {
        self.free_command_buffers();
        self.destroy_framebuffers();

        self.device.handle().destroy_pipeline(self.graphics_pipeline, None);
        log::debug!("dropped pipeline {:?}", self.graphics_pipeline);

        self.deferred_render_pass.destroy();
        self.pipeline_layout.destroy();
        self.vertex_shader_module.destroy();
        self.fragment_shader_module.destroy();
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

    pub fn recreate_frame_buffers(&mut self, swapchain: &Swapchain) -> Result<()> {
        // destroy depth textures and framebuffers
        unsafe {
            self.destroy_framebuffers();
        };

        // create framebuffers
        self.framebuffers = swapchain
            .image_views()
            .iter()
            .map(|image_view| {
                let extent = swapchain.extent();

                let depth_image = Image::new(
                    self.device.clone(),
                    [extent.width, extent.height],
                    1,
                    vk::SampleCountFlags::TYPE_1,
                    self.depth_format,
                    vk::ImageTiling::OPTIMAL,
                    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                )?;

                let depth_image_view = ImageView::new(
                    self.device.clone(),
                    &depth_image,
                    self.depth_format,
                    vk::ImageAspectFlags::DEPTH,
                    1,
                )?;

                let framebuffer = Framebuffer::new(
                    self.device.clone(),
                    self.deferred_render_pass.handle(),
                    &[image_view.handle(), depth_image_view.handle()],
                    extent,
                )?;

                Ok((framebuffer, depth_image, depth_image_view))
            })
            .collect::<Result<_>>()?;

        // done
        Ok(())
    }

    pub fn recreate_command_buffers(&mut self, swapchain: &Swapchain) -> Result<()> {
        // free command buffers
        unsafe { self.free_command_buffers() };

        let extent = swapchain.extent();

        // create command buffers
        let device = self.device.handle();

        let command_buffer_create_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool.handle())
            .command_buffer_count(swapchain.image_count())
            .level(vk::CommandBufferLevel::PRIMARY);

        self.command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_create_info)? };

        let viewports = [utils::viewport_flipped(extent, 0.0, 1.0)];
        let scissors = [utils::rect_2d([0, 0], extent)];

        for (i, &command_buffer) in self.command_buffers.iter().enumerate() {
            let command_buffer_begin_info =
                vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

            unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info)? }

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 },
                },
            ];

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.deferred_render_pass.handle())
                .framebuffer(self.framebuffers[i].0.handle())
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

    #[inline]
    pub fn command_buffer(&self, image_index: usize) -> vk::CommandBuffer {
        self.command_buffers[image_index]
    }

    #[allow(unused)]
    #[inline]
    pub fn pipeline_layout(&self) -> &GraphicsPipelineLayout {
        &self.pipeline_layout
    }

    #[inline]
    pub fn pipeline_layout_mut(&mut self) -> &mut GraphicsPipelineLayout {
        &mut self.pipeline_layout
    }
}
