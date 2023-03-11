use std::{ffi::CString, sync::Arc};

use anyhow::Result;
use ash::vk;

use crate::{device::Device, Context};

use crate::{PipelineLayout, RayTracingContext, ShaderModule};

#[derive(Debug, Clone, Copy)]
pub struct RayTracingPipelineCreateInfo<'a> {
    pub shaders: &'a [RayTracingShaderCreateInfo<'a>],
    pub max_ray_recursion_depth: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct RayTracingShaderCreateInfo<'a> {
    pub source: &'a [u8],
    pub stage: vk::ShaderStageFlags,
    pub group: RayTracingShaderGroup,
}

#[derive(Debug, Clone, Copy)]
pub enum RayTracingShaderGroup {
    RayGen,
    Miss,
    ClosestHit,
}

pub struct RayTracingPipeline {
    device: Arc<Device>,
    pub(crate) inner: vk::Pipeline,
    pub(crate) shader_group_info: RayTracingShaderGroupInfo,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RayTracingShaderGroupInfo {
    pub group_count: u32,
    pub raygen_shader_count: u32,
    pub miss_shader_count: u32,
    pub hit_shader_count: u32,
}

impl RayTracingPipeline {
    pub(crate) fn new(
        device: Arc<Device>,
        ray_tracing: &RayTracingContext,
        layout: &PipelineLayout,
        create_info: RayTracingPipelineCreateInfo,
    ) -> Result<Self> {
        let mut shader_group_info = RayTracingShaderGroupInfo {
            group_count: create_info.shaders.len() as _,
            ..Default::default()
        };

        let mut modules = vec![];
        let mut stages = vec![];
        let mut groups = vec![];

        let entry_point_name = CString::new("main").unwrap();

        for (shader_index, shader) in create_info.shaders.iter().enumerate() {
            let module = ShaderModule::from_bytes(device.clone(), shader.source)?;

            let stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(shader.stage)
                .module(module.inner)
                .name(&entry_point_name)
                .build();

            match shader.group {
                RayTracingShaderGroup::RayGen => shader_group_info.raygen_shader_count += 1,
                RayTracingShaderGroup::Miss => shader_group_info.miss_shader_count += 1,
                RayTracingShaderGroup::ClosestHit => shader_group_info.hit_shader_count += 1,
            };

            let mut group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR);
            group = match shader.group {
                RayTracingShaderGroup::RayGen | RayTracingShaderGroup::Miss => {
                    group.general_shader(shader_index as _)
                }
                RayTracingShaderGroup::ClosestHit => group
                    .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                    .closest_hit_shader(shader_index as _),
            };

            modules.push(module);
            stages.push(stage);
            groups.push(group.build());
        }

        let pipe_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .layout(layout.inner)
            .stages(&stages)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(2);

        let inner = unsafe {
            ray_tracing.pipeline_fn.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(),
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipe_info),
                None,
            )?[0]
        };

        Ok(Self {
            device,
            inner,
            shader_group_info,
        })
    }
}

impl Context {
    pub fn create_ray_tracing_pipeline(
        &self,
        layout: &PipelineLayout,
        create_info: RayTracingPipelineCreateInfo,
    ) -> Result<RayTracingPipeline> {
        let ray_tracing = self.ray_tracing.as_ref().expect(
            "Cannot call Context::create_ray_tracing_pipeline when ray tracing is not enabled",
        );

        RayTracingPipeline::new(self.device.clone(), ray_tracing, layout, create_info)
    }
}

impl Drop for RayTracingPipeline {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_pipeline(self.inner, None) };
    }
}
