use std::{any, ffi::CString, sync::Arc};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RayTracingShaderGroup {
    RayGen,
    Miss,
    ClosestHit,
    AnyHit,
    ShadowAnyHit,
    ShadowMiss,
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


        let mut modules = vec![];
        let mut stages = vec![];
        let mut groups = vec![];

        let entry_point_name = CString::new("main").unwrap();
        let mut anyhit = None;
        let mut shadow_anyhit = None;
        create_info.shaders.iter().enumerate()
            .for_each(|(index, shader)|
                {
                    match shader.group {
                        RayTracingShaderGroup::AnyHit => {
                            assert!(anyhit.is_none());
                            anyhit = Some(index);
                        }
                        RayTracingShaderGroup::ShadowAnyHit => {
                            assert!(shadow_anyhit.is_none());
                            shadow_anyhit = Some(index);
                        },
                        _ => {}
                    }
                }
            );
        let mut shader_group_info = RayTracingShaderGroupInfo {
            group_count:  create_info.shaders.len() as u32 - if anyhit.is_some() { 1 } else { 0 },
            ..Default::default()
        };
        for (shader_index, shader) in create_info.shaders.iter().enumerate() {
            let module = ShaderModule::from_bytes(device.clone(), shader.source)?;

            let stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(shader.stage)
                .module(module.inner)
                .name(&entry_point_name)
                .build();

            match shader.group {
                RayTracingShaderGroup::RayGen => shader_group_info.raygen_shader_count += 1,
                RayTracingShaderGroup::Miss | RayTracingShaderGroup::ShadowMiss => shader_group_info.miss_shader_count += 1,
                RayTracingShaderGroup::ClosestHit |  RayTracingShaderGroup::ShadowAnyHit =>
                    shader_group_info.hit_shader_count += 1,
                _ => {}
            };
            if shader.group != RayTracingShaderGroup::AnyHit {
                let mut group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                    .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR);
                group = match shader.group {
                    RayTracingShaderGroup::RayGen | RayTracingShaderGroup::Miss
                    | RayTracingShaderGroup::ShadowMiss
                    => {
                        group.general_shader(shader_index as _)
                    }
                    RayTracingShaderGroup::ClosestHit
                    => {
                        group
                            .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                            .closest_hit_shader(shader_index as _)
                            .any_hit_shader(if let Some(anyhit_index) = anyhit {anyhit_index as _}
                                            else {vk::SHADER_UNUSED_KHR}
                            )
                    },

                    RayTracingShaderGroup::ShadowAnyHit => group
                        .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                        .any_hit_shader(shader_index as _),
                    RayTracingShaderGroup::AnyHit
                    => {unreachable!()}
                    // _ => {}
                };
                groups.push(group.build());
            }

            modules.push(module);
            stages.push(stage);
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
