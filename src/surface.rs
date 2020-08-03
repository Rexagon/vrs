use anyhow::Result;
use ash::vk;

pub struct Surface {
    surface_ext: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance, window: &winit::window::Window) -> Result<Self> {
        let surface_ext = ash::extensions::khr::Surface::new(entry, instance);
        let surface = unsafe { ash_window::create_surface(entry, instance, window, None)? };
        log::debug!("created surface: {:?}", surface);

        Ok(Self { surface_ext, surface })
    }

    #[inline]
    pub fn ext(&self) -> &ash::extensions::khr::Surface {
        &self.surface_ext
    }

    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    pub unsafe fn destroy(&self) {
        self.surface_ext.destroy_surface(self.surface, None);
        log::debug!("dropped surface");
    }
}
