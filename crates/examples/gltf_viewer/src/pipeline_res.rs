use crate::{
    ACC_BIND, AS_BIND, DLIGHT_BIND, GEO_BIND, INDEX_BIND, MAT_BIND, PLIGHT_BIND, SKYBOX_BIND,
    STORAGE_BIND, TEXTURE_BIND, UNIFORM_BIND, VERTEX_BIND,
};
use app::anyhow::Result;
use app::load_spv;
use app::vulkan::ash::vk;
use app::vulkan::{
    Context, DescriptorSetLayout, PipelineLayout, RayTracingPipeline, RayTracingPipelineCreateInfo,
    RayTracingShaderCreateInfo, RayTracingShaderGroup,
};
use asset_loader::globals::VkGlobal;

pub struct PipelineRes {
    pub(crate) pipeline: RayTracingPipeline,
    pub(crate) pipeline_layout: PipelineLayout,
    pub(crate) static_dsl: DescriptorSetLayout,
    pub(crate) dynamic_dsl: DescriptorSetLayout,
}

pub fn create_pipeline(
    context: &Context,
    model: &VkGlobal,
    fully_opaque: bool,
) -> Result<PipelineRes> {
    // descriptor and pipeline layouts
    let primary_hit_group_flags =
        vk::ShaderStageFlags::ANY_HIT_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR;
    let static_layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(AS_BIND)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(UNIFORM_BIND)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(
                vk::ShaderStageFlags::RAYGEN_KHR
                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR
                    | vk::ShaderStageFlags::MISS_KHR,
            )
            .build(),
        // Vertex buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(VERTEX_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(primary_hit_group_flags)
            .build(),
        //Index buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(INDEX_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(primary_hit_group_flags)
            .build(),
        // Geometry info buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(GEO_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(primary_hit_group_flags)
            .build(),
        // Textures
        vk::DescriptorSetLayoutBinding::builder()
            .binding(TEXTURE_BIND)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.textures.len() as _)
            .stage_flags(primary_hit_group_flags)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(MAT_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(primary_hit_group_flags)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(DLIGHT_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::MISS_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(PLIGHT_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(SKYBOX_BIND)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::MISS_KHR)
            .build(),
    ];

    let dynamic_layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(STORAGE_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(ACC_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
            .build(),
    ];

    let static_dsl = context.create_descriptor_set_layout(&static_layout_bindings)?;
    let dynamic_dsl = context.create_descriptor_set_layout(&dynamic_layout_bindings)?;
    let dsls = [&static_dsl, &dynamic_dsl];

    let pipeline_layout = context.create_pipeline_layout(&dsls)?;

    // Shaders
    let ray_gen = load_spv("RayTracing.rgen.spv");
    let ray_miss = load_spv("RayTracing.rmiss.spv");
    let ray_chit = load_spv("RayTracing.rchit.spv");
    let ray_rahit = load_spv("RayTracing.rahit.spv");
    let _shadow_rahit = load_spv("RayTracing.rahit.spv");
    let shadow_miss = load_spv("RayTracing.shadow.rmiss.spv");
    let mut shaders_create_info = vec![
        RayTracingShaderCreateInfo {
            source: &ray_gen,
            stage: vk::ShaderStageFlags::RAYGEN_KHR,
            group: RayTracingShaderGroup::RayGen,
        },
        RayTracingShaderCreateInfo {
            source: &ray_miss,
            stage: vk::ShaderStageFlags::MISS_KHR,
            group: RayTracingShaderGroup::Miss,
        },
        RayTracingShaderCreateInfo {
            source: &shadow_miss,
            stage: vk::ShaderStageFlags::MISS_KHR,
            group: RayTracingShaderGroup::Miss,
        },
        // RayTracingShaderCreateInfo {
        //     source: &include_bytes!("../spv/shadow.rmiss.spv")[..],
        //     stage: vk::ShaderStageFlags::MISS_KHR,
        //     group: RayTracingShaderGroup::Miss,
        // },
        RayTracingShaderCreateInfo {
            source: &ray_chit,
            stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            group: RayTracingShaderGroup::ClosestHit,
        },
        // RayTracingShaderCreateInfo {
        //     source: &shadow_rahit,
        //     stage: vk::ShaderStageFlags::ANY_HIT_KHR,
        //     group: RayTracingShaderGroup::ShadowAnyHit,
        // },
    ];
    if !fully_opaque {
        shaders_create_info.push(RayTracingShaderCreateInfo {
            source: &ray_rahit,
            stage: vk::ShaderStageFlags::ANY_HIT_KHR,
            group: RayTracingShaderGroup::AnyHit,
        });
    }

    let pipeline_create_info = RayTracingPipelineCreateInfo {
        shaders: shaders_create_info.as_slice(),
        max_ray_recursion_depth: 2,
    };

    let pipeline = context.create_ray_tracing_pipeline(&pipeline_layout, pipeline_create_info)?;

    Ok(PipelineRes {
        pipeline,
        pipeline_layout,
        static_dsl,
        dynamic_dsl,
    })
}
//
