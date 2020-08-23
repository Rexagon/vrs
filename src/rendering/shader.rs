use super::prelude::*;
use super::{utils, Device};

pub struct ShaderModule {
    device: Arc<Device>,
    shader_module: vk::ShaderModule,
}

impl ShaderModule {
    pub fn from_file<T>(device: Arc<Device>, path: T) -> Result<Self>
    where
        T: AsRef<std::path::Path>,
    {
        let code = utils::read_shader_code(path)?;
        Self::new(device, &code)
    }

    pub fn new(device: Arc<Device>, code: &[u8]) -> Result<Self> {
        let shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(bytemuck::cast_slice(code));

        let shader_module = unsafe { device.handle().create_shader_module(&shader_module_create_info, None)? };
        log::debug!("created shader module {:?}", shader_module);

        Ok(Self { device, shader_module })
    }

    pub unsafe fn destroy(&self) {
        self.device.handle().destroy_shader_module(self.shader_module, None);
        log::debug!("dropped shader module {:?}", self.shader_module);
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.shader_module
    }
}

pub fn main_function_name() -> &'static CStr {
    MAIN_FUNCTION_NAME
        .get_or_init(|| CString::new("main").unwrap())
        .as_c_str()
}

static MAIN_FUNCTION_NAME: OnceCell<CString> = OnceCell::new();
