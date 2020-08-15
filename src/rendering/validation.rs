use super::prelude::*;
use super::Instance;

pub struct Validation {
    is_enabled: bool,
    debug_utils_ext: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl Validation {
    pub fn new(entry: &ash::Entry, instance: &Instance, is_enabled: bool) -> Result<Self> {
        let debug_utils_ext = ash::extensions::ext::DebugUtils::new(entry, instance.handle());

        let debug_utils_messenger = if is_enabled {
            let debug_utils_messenger =
                unsafe { debug_utils_ext.create_debug_utils_messenger(&debug_messenger_create_info(), None)? };
            log::debug!("created debug utils messenger: {:?}", debug_utils_messenger);

            debug_utils_messenger
        } else {
            vk::DebugUtilsMessengerEXT::null()
        };

        Ok(Self {
            is_enabled,
            debug_utils_ext,
            debug_utils_messenger,
        })
    }

    pub unsafe fn destroy(&self) {
        if self.is_enabled {
            self.debug_utils_ext
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            log::debug!("dropped debug utils messenger");
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    #[allow(unused)]
    #[inline]
    pub fn ext(&self) -> &ash::extensions::ext::DebugUtils {
        &self.debug_utils_ext
    }
}

pub fn check_supported(entry: &ash::Entry) -> Result<()> {
    let layer_properties = entry.enumerate_instance_layer_properties()?;

    if layer_properties.is_empty() {
        return Err(Error::msg("no available layers found"));
    }

    for required_layer_name in required_layers().iter() {
        let mut is_layer_found = false;

        for layer_property in layer_properties.iter() {
            let layer_name = unsafe { CStr::from_ptr(layer_property.layer_name.as_ptr()) };

            if required_layer_name.as_c_str() == layer_name {
                is_layer_found = true;
                break;
            }
        }

        if !is_layer_found {
            return Err(Error::msg(format!(
                "required layer {:?} was not found",
                required_layer_name
            )));
        }
    }

    Ok(())
}

pub fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback))
        .build()
}

pub fn required_layers() -> &'static [CString] {
    REQUIRED_LAYERS.get_or_init(|| vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()])
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        _ => "[UNKNOWN]",
    };

    let message = CStr::from_ptr((*p_callback_data).p_message);
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::debug!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::warn!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::warn!("{} {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::info!("{} {:?}", message_type, message),
        _ => log::trace!("{} {:?}", message_type, message),
    }

    vk::FALSE
}

static REQUIRED_LAYERS: OnceCell<Vec<CString>> = OnceCell::new();
