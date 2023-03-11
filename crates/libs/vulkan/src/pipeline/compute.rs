use std::{ffi::CString, sync::Arc};

use anyhow::Result;
use ash::vk;

use crate::{device::Device, Context, PipelineLayout, ShaderModule};

pub struct ComputePipeline {
    device: Arc<Device>,
    pub(crate) inner: vk::Pipeline,
}

#[derive(Debug, Clone, Copy)]
pub struct ComputePipelineCreateInfo<'a> {
    pub shader_source: &'a [u8],
}

impl ComputePipeline {
    pub(crate) fn new(
        device: Arc<Device>,
        layout: &PipelineLayout,
        create_info: ComputePipelineCreateInfo,
    ) -> Result<Self> {
        let entry_point_name = CString::new("main").unwrap();
        let shader_module = ShaderModule::from_bytes(device.clone(), create_info.shader_source)?;
        let shader_stage_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module.inner)
            .name(&entry_point_name)
            .build();

        let pipeline_info = vk::ComputePipelineCreateInfo::builder()
            .stage(shader_stage_info)
            .layout(layout.inner);

        let inner = unsafe {
            device
                .inner
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|e| e.1)?[0]
        };

        Ok(Self { device, inner })
    }
}

impl Context {
    pub fn create_compute_pipeline(
        &self,
        layout: &PipelineLayout,
        create_info: ComputePipelineCreateInfo,
    ) -> Result<ComputePipeline> {
        ComputePipeline::new(self.device.clone(), layout, create_info)
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_pipeline(self.inner, None) };
    }
}
