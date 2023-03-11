use app::vulkan::{Context, DescriptorSetLayout, PipelineLayout, RayTracingPipeline, RayTracingPipelineCreateInfo, RayTracingShaderCreateInfo, RayTracingShaderGroup};
use crate::model::Model;
use app::anyhow::Result;
use app::vulkan::ash::vk;
use crate::ACC_BIND;

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
            .binding(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR | vk::ShaderStageFlags::MISS_KHR)
            .build(),
        // Vertex buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        //Index buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Geometry info buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(5)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Textures
        vk::DescriptorSetLayoutBinding::builder()
            .binding(6)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.images.len() as _)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build()
    ];

    let dynamic_layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
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
    let shaders_create_info = [
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../spv/RayTracing.rgen.spv")[..],
            stage: vk::ShaderStageFlags::RAYGEN_KHR,
            group: RayTracingShaderGroup::RayGen,
        },
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../spv/RayTracing.rmiss.spv")[..],
            stage: vk::ShaderStageFlags::MISS_KHR,
            group: RayTracingShaderGroup::Miss,
        },
        // RayTracingShaderCreateInfo {
        //     source: &include_bytes!("../spv/shadow.rmiss.spv")[..],
        //     stage: vk::ShaderStageFlags::MISS_KHR,
        //     group: RayTracingShaderGroup::Miss,
        // },
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../spv/RayTracing.rchit.spv")[..],
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
