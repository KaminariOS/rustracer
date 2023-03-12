use app::vulkan::{Context, DescriptorSetLayout, PipelineLayout, RayTracingPipeline, RayTracingPipelineCreateInfo, RayTracingShaderCreateInfo, RayTracingShaderGroup};
use crate::model::Model;
use app::anyhow::Result;
use app::load_spv;
use app::vulkan::ash::vk;
use crate::{ACC_BIND, AS_BIND, GEO_BIND, INDEX_BIND, STORAGE_BIND, TEXTURE_BIND, UNIFORM_BIND, VERTEX_BIND};

pub struct PipelineRes {
    pub(crate) pipeline: RayTracingPipeline,
    pub(crate) pipeline_layout: PipelineLayout,
    pub(crate) static_dsl: DescriptorSetLayout,
    pub(crate) dynamic_dsl: DescriptorSetLayout,
}

pub fn create_pipeline(context: &Context, model: &Model) -> Result<PipelineRes> {
    // descriptor and pipeline layouts
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
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR | vk::ShaderStageFlags::MISS_KHR)
            .build(),
        // Vertex buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(VERTEX_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        //Index buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(INDEX_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Geometry info buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(GEO_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Textures
        vk::DescriptorSetLayoutBinding::builder()
            .binding(TEXTURE_BIND)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.textures.len() as _)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build()
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
            .build()
    ];

    let static_dsl = context.create_descriptor_set_layout(&static_layout_bindings)?;
    let dynamic_dsl = context.create_descriptor_set_layout(&dynamic_layout_bindings)?;
    let dsls = [&static_dsl, &dynamic_dsl];

    let pipeline_layout = context.create_pipeline_layout(&dsls)?;


    // Shaders
    let ray_gen = load_spv("RayTracing.rgen.spv");
    let ray_miss = load_spv("RayTracing.rmiss.spv");
    let ray_chit = load_spv("RayTracing.rchit.spv");
    let shaders_create_info = [
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
    ];

    let pipeline_create_info = RayTracingPipelineCreateInfo {
        shaders: &shaders_create_info,
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
// pub fn create_ras_pipeline(context: &Context, model: &Model) {
//
// }
