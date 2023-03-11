use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use crate::{device::Device, Context};

pub struct Sampler {
    device: Arc<Device>,
    pub(crate) inner: vk::Sampler,
}

impl Sampler {
    pub(crate) fn new(device: Arc<Device>, create_info: &vk::SamplerCreateInfo) -> Result<Self> {
        let inner = unsafe { device.inner.create_sampler(create_info, None)? };

        Ok(Self { device, inner })
    }
}

impl Context {
    pub fn create_sampler(&self, create_info: &vk::SamplerCreateInfo) -> Result<Sampler> {
        Sampler::new(self.device.clone(), create_info)
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device.inner.destroy_sampler(self.inner, None);
        }
    }
}
