use super::prelude::*;

#[allow(unused)]
pub fn from_vk_string_raw(raw_string_array: &[c_char]) -> &CStr {
    unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    }
}

#[allow(unused)]
pub fn from_vk_string(raw_string_array: &[c_char]) -> String {
    from_vk_string_raw(raw_string_array).to_str().unwrap().to_owned()
}

#[allow(unused)]
pub fn checked_from_vk_string(raw_string_array: &[c_char]) -> Result<String> {
    Ok(from_vk_string_raw(raw_string_array).to_str()?.to_owned())
}

#[allow(unused)]
pub fn as_ptr_vec(names: &[CString]) -> Vec<*const c_char> {
    names.iter().map(|item| item.as_ptr()).collect()
}

#[allow(unused)]
pub fn read_shader_code<T>(path: T) -> Result<Vec<u8>>
where
    T: AsRef<Path>,
{
    let bytes = std::fs::read(path)?;
    Ok(bytes)
}

#[allow(unused)]
pub fn viewport(extent: vk::Extent2D, min_depth: f32, max_depth: f32) -> vk::Viewport {
    vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: extent.width as f32,
        height: extent.height as f32,
        min_depth,
        max_depth,
    }
}

#[allow(unused)]
pub fn rect_2d(offset: [i32; 2], extent: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D {
            x: offset[0],
            y: offset[1],
        },
        extent,
    }
}
