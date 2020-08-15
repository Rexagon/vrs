pub mod buffer;
pub mod command_buffer;
pub mod device;
pub mod frame;
pub mod framebuffer;
pub mod instance;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod utils;
pub mod validation;

pub use self::buffer::Buffer;
pub use self::command_buffer::CommandPool;
pub use self::device::Device;
pub use self::frame::{Frame, FrameLogic, FrameSyncObjects};
pub use self::framebuffer::Framebuffer;
pub use self::instance::Instance;
pub use self::mesh::{Mesh, Vertex};
pub use self::pipeline::PipelineCache;
pub use self::shader::ShaderModule;
pub use self::surface::Surface;
pub use self::swapchain::Swapchain;
pub use self::validation::Validation;

pub(self) mod prelude {
    pub use std::collections::HashSet;
    pub use std::ffi::{c_void, CStr, CString};
    pub use std::os::raw::c_char;
    pub use std::path::Path;

    pub use anyhow::{Error, Result};
    pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
    pub use ash::vk;
    pub use once_cell::sync::OnceCell;
    pub use winit::window::Window;
}
