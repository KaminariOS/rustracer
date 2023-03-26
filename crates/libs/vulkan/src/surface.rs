use crate::utils::platforms;
use anyhow::Result;
use ash::{extensions::khr::Surface as AshSurface, vk, Entry};

use crate::instance::Instance;

pub struct Surface {
    pub(crate) inner: AshSurface,
    pub surface_khr: vk::SurfaceKHR,
}

impl Surface {
    pub(crate) fn new(
        entry: &Entry,
        instance: &Instance,
        window: &winit::window::Window,
    ) -> Result<Self> {
        let inner = AshSurface::new(entry, &instance.inner);
        let surface_khr = unsafe {
            platforms::create_surface(entry, &instance.inner, window)
                .expect("Failed to create surface.")
        };

        Ok(Self { inner, surface_khr })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_surface(self.surface_khr, None);
        }
    }
}
