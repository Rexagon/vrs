use anyhow::{Error, Result};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::logical_device::LogicalDevice;
use crate::shader::{self, ShaderModule};
use crate::utils;

pub struct DefaultPipeline {
    pipeline_layout: vk::PipelineLayout,
    vertex_shader_module: ShaderModule,
    fragment_shader_module: ShaderModule,
}

impl DefaultPipeline {
    pub fn new(logical_device: &LogicalDevice, extent: vk::Extent2D) -> Result<Self> {
        let vertex_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.vert.spv")?;
        let fragment_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.frag.spv")?;

        let main_function_name = shader::main_function_name();

        // shader stages
        let mut shader_stages = Vec::new();
        shader_stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .module(vertex_shader_module.handle())
                .name(main_function_name)
                .stage(vk::ShaderStageFlags::VERTEX),
        );
        shader_stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .module(fragment_shader_module.handle())
                .name(main_function_name)
                .stage(vk::ShaderStageFlags::FRAGMENT),
        );

        // vertex input state
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&[])
            .vertex_binding_descriptions(&[]);

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
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

        let depth_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
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

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder();

        let pipeline_layout = unsafe {
            logical_device
                .device()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };
        log::debug!("created pipeline layout {:?}", pipeline_layout);

        Ok(Self {
            pipeline_layout,
            vertex_shader_module,
            fragment_shader_module,
        })
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device
            .device()
            .destroy_pipeline_layout(self.pipeline_layout, None);
        log::debug!("dropped pipeline layout {:?}", self.pipeline_layout);

        self.vertex_shader_module.destroy(logical_device);
        self.fragment_shader_module.destroy(logical_device);
    }
}
