use std::ffi::CStr;
use std::os::raw::c_char;

use anyhow::Result;

#[allow(unused)]
pub fn from_vk_string(raw_string_array: &[c_char]) -> String {
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    raw_string.to_str().unwrap().to_owned()
}

#[allow(unused)]
pub fn checked_from_vk_string(raw_string_array: &[c_char]) -> Result<String> {
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    Ok(raw_string.to_str()?.to_owned())
}
