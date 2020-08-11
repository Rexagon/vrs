use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::logical_device::LogicalDevice;
use crate::shader::{self, ShaderModule};

pub struct DefaultPipeline {
    graphics_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    vertex_shader_module: ShaderModule,
    fragment_shader_module: ShaderModule,
}

impl DefaultPipeline {
    pub fn new(logical_device: &LogicalDevice, extent: vk::Extent2D, render_pass: &SimpleRenderPass) -> Result<Self> {
        let vertex_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.vert.spv")?;
        let fragment_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.frag.spv")?;

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
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&[])
            .vertex_binding_descriptions(&[]);

        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // viewports
        let viewports = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];

        let scissorts = [vk::Rect2D::builder().extent(extent).build()];

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissorts)
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
            .compare_op(vk::CompareOp::ALWAYS)
            .compare_mask(0)
            .write_mask(0)
            .reference(0)
            .build();

        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
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

        // layout creation info
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder();

        let pipeline_layout = unsafe {
            logical_device
                .handle()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };
        log::debug!("created pipeline layout {:?}", pipeline_layout);

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
            .layout(pipeline_layout)
            .render_pass(render_pass.handle())
            .subpass(0)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(-1)
            .build()];

        let graphics_pipelines = unsafe {
            logical_device
                .handle()
                .create_graphics_pipelines(vk::PipelineCache::null(), &graphics_pipeline_create_infos, None)
                .map_err(|(_, e)| e)?
        };
        let graphics_pipeline = graphics_pipelines[0];
        log::debug!("create graphics pipeline {:?}", graphics_pipeline);

        Ok(Self {
            graphics_pipeline,
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
        })
    }

    #[inline]
    pub fn handle(&self) -> vk::Pipeline {
        self.graphics_pipeline
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.handle().destroy_pipeline(self.graphics_pipeline, None);
        log::debug!("dropped graphics pipeline {:?}", self.graphics_pipeline);

        logical_device
            .handle()
            .destroy_pipeline_layout(self.pipeline_layout, None);
        log::debug!("dropped pipeline layout {:?}", self.pipeline_layout);

        self.vertex_shader_module.destroy(logical_device);
        self.fragment_shader_module.destroy(logical_device);
    }
}

pub struct SimpleRenderPass {
    render_pass: vk::RenderPass,
}

impl SimpleRenderPass {
    pub fn new(logical_device: &LogicalDevice, surface_format: vk::Format) -> Result<Self> {
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

        let render_pass = unsafe {
            logical_device
                .handle()
                .create_render_pass(&render_pass_create_info, None)?
        };
        log::debug!("created render pass {:?}", render_pass);

        Ok(Self { render_pass })
    }

    #[inline]
    pub fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.handle().destroy_render_pass(self.render_pass, None);
        log::debug!("dropped render pass {:?}", self.render_pass);
    }
}
