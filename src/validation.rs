use std::ffi::{c_void, CStr};

use anyhow::{Error, Result};
use ash::version::EntryV1_0;
use ash::vk;

use crate::utils;

pub struct ValidationInfo {
    pub is_enabled: bool,
    pub required_validation_layers: [&'static str; 1],
}

pub fn check_validation_error_support(entry: &ash::Entry, validation_info: &ValidationInfo) -> Result<()> {
    if !validation_info.is_enabled {
        return Ok(());
    }

    let layer_properties = entry.enumerate_instance_layer_properties()?;

    if layer_properties.is_empty() {
        return Err(Error::msg("no available layers found"));
    }

    for required_layer_name in validation_info.required_validation_layers.iter() {
        let mut is_layer_found = false;

        for layer_property in layer_properties.iter() {
            let test_layer_name = utils::from_vk_string(&layer_property.layer_name);
            if *required_layer_name == test_layer_name {
                is_layer_found = true;
                break;
            }
        }

        if is_layer_found == false {
            return Err(Error::msg(format!(
                "required layer `{}` was not found",
                required_layer_name
            )));
        }
    }

    Ok(())
}

pub fn create_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
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

pub fn setup_debug_utils(
    entry: &ash::Entry,
    instance: &ash::Instance,
    debug_utils_messenger_create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    validation_info: &ValidationInfo,
) -> Result<(ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT)> {
    let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

    if validation_info.is_enabled {
        let messenger =
            unsafe { debug_utils_loader.create_debug_utils_messenger(debug_utils_messenger_create_info, None)? };
        log::debug!("created debug utils messenger: {:?}", messenger);

        Ok((debug_utils_loader, messenger))
    } else {
        Ok((debug_utils_loader, vk::DebugUtilsMessengerEXT::null()))
    }
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
