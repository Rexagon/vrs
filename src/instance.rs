use std::ffi::{CStr, CString};

use anyhow::Result;
use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;
use winit::window::Window;

use crate::validation;

pub const APPLICATION_NAME: &str = "vrs";
pub const ENGINE_TITLE: &str = "ash";
pub const APPLICATION_VERSION: u32 = vk::make_version(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk::make_version(1, 0, 0);

pub struct Instance {
    instance: ash::Instance,
}

impl Instance {
    pub fn new(entry: &ash::Entry, window: &Window, is_validation_enabled: bool) -> Result<Self> {
        if is_validation_enabled {
            validation::check_supported(entry)?;
        }

        let application_name = CString::new(APPLICATION_NAME)?;
        let engine_name = CString::new(ENGINE_TITLE)?;

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&application_name)
            .engine_name(&engine_name)
            .application_version(APPLICATION_VERSION)
            .engine_version(ENGINE_VERSION);

        //
        let mut required_extensions = ash_window::enumerate_required_extensions(window)?;
        required_extensions.push(ash::extensions::ext::DebugUtils::name());

        required_extensions.iter().for_each(|extension| {
            log::debug!("required extension: {:?}", extension);
        });

        let required_extensions = required_extensions.into_iter().map(CStr::as_ptr).collect::<Vec<_>>();

        //
        let required_layers = if is_validation_enabled {
            validation::required_layers()
        } else {
            &[]
        };

        required_layers.iter().for_each(|layer| {
            log::debug!("required layer: {:?}", layer);
        });

        let required_layers: Vec<*const i8> = required_layers
            .into_iter()
            .map(|item| item.as_ptr())
            .collect::<Vec<_>>();

        //
        let mut instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&required_extensions)
            .enabled_layer_names(&required_layers);

        //
        let mut debug_utils_creation_info = validation::debug_messenger_create_info();
        if is_validation_enabled {
            instance_info = instance_info.push_next(&mut debug_utils_creation_info);
        }

        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        log::debug!("created instance");

        Ok(Self { instance })
    }

    #[inline]
    pub fn get(&self) -> &ash::Instance {
        &self.instance
    }

    pub unsafe fn destroy(&self) {
        self.instance.destroy_instance(None);
        log::debug!("dropped instance");
    }
}
