use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use crate::{device::Device, utils::read_shader_from_bytes, Context};

pub struct ShaderModule {
    device: Arc<Device>,
    pub(crate) inner: vk::ShaderModule,
}

impl ShaderModule {
    pub(crate) fn from_bytes(device: Arc<Device>, source: &[u8]) -> Result<Self> {
        let source = read_shader_from_bytes(source)?;

        let create_info = vk::ShaderModuleCreateInfo::builder().code(&source);
        let inner = unsafe { device.inner.create_shader_module(&create_info, None)? };

        Ok(Self { device, inner })
    }
}

impl Context {
    pub fn create_shader_module(&self, source: &[u8]) -> Result<ShaderModule> {
        ShaderModule::from_bytes(self.device.clone(), source)
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.inner.destroy_shader_module(self.inner, None);
        }
    }
}
