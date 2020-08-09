use std::ffi::{CStr, CString};

use anyhow::{Error, Result};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use once_cell::*;

use crate::logical_device::LogicalDevice;
use crate::utils;

pub fn create_graphics_pipeline(logical_device: &LogicalDevice) -> Result<()> {
    let vertex_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.vert.spv")?;
    let fragment_shader_module = ShaderModule::from_file(logical_device, "shaders/spv/mesh.frag.spv")?;

    let main_function_name = main_function_name();

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

    unsafe {
        vertex_shader_module.destroy(logical_device);
        fragment_shader_module.destroy(logical_device);
    }

    Ok(())
}

pub struct ShaderModule {
    shader_module: vk::ShaderModule,
}

impl ShaderModule {
    pub fn from_file<T>(logical_device: &LogicalDevice, path: T) -> Result<Self>
    where
        T: AsRef<std::path::Path>,
    {
        let code = utils::read_shader_code(path)?;
        Self::new(logical_device, &code)
    }

    pub fn new(logical_device: &LogicalDevice, code: &[u8]) -> Result<Self> {
        let shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(bytemuck::cast_slice(code));

        let shader_module = unsafe {
            logical_device
                .device()
                .create_shader_module(&shader_module_create_info, None)?
        };
        log::debug!("created shader module {:?}", shader_module);

        Ok(Self { shader_module })
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.shader_module
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        logical_device.device().destroy_shader_module(self.shader_module, None);
        log::debug!("dropped shader module {:?}", self.shader_module);
    }
}

pub fn main_function_name() -> &'static CStr {
    MAIN_FUNCTION_NAME
        .get_or_init(|| CString::new("main").unwrap())
        .as_c_str()
}

static MAIN_FUNCTION_NAME: sync::OnceCell<CString> = sync::OnceCell::new();
