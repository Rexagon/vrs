use super::prelude::*;
use super::Instance;

pub struct Surface {
    surface_ext: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn new(entry: &ash::Entry, instance: &Instance, window: &winit::window::Window) -> Result<Self> {
        let surface_ext = ash::extensions::khr::Surface::new(entry, instance.handle());
        let surface = unsafe { ash_window::create_surface(entry, instance.handle(), window, None)? };
        log::debug!("created surface: {:?}", surface);

        Ok(Self { surface_ext, surface })
    }

    pub unsafe fn destroy(&self) {
        self.surface_ext.destroy_surface(self.surface, None);
        log::debug!("dropped surface");
    }

    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    #[inline]
    pub fn ext(&self) -> &ash::extensions::khr::Surface {
        &self.surface_ext
    }
}
