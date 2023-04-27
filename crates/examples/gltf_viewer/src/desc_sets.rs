use crate::pipeline_res::PipelineRes;
use crate::{
    ACC_BIND, AS_BIND, DLIGHT_BIND, GEO_BIND, INDEX_BIND, MAT_BIND, PLIGHT_BIND, STORAGE_BIND,
    TEXTURE_BIND, UNIFORM_BIND, VERTEX_BIND,
};
use app::anyhow::Result;
use app::vulkan::ash::vk;
use app::vulkan::{
    Buffer, Context, DescriptorPool, DescriptorSet, WriteDescriptorSet, WriteDescriptorSetKind,
};
use app::ImageAndView;
use asset_loader::acceleration_structures::TopAS;
use asset_loader::globals::{Buffers, VkGlobal};

pub struct DescriptorRes {
    pub(crate) _pool: DescriptorPool,
    pub(crate) static_set: DescriptorSet,
    pub(crate) dynamic_sets: Vec<DescriptorSet>,
}

pub fn create_descriptor_sets(
    context: &Context,
    pipeline_res: &PipelineRes,
    model: &VkGlobal,
    top_as: &TopAS,
    storage_imgs: &[ImageAndView],
    acc_images: &[ImageAndView],
    ubo_buffer: &Buffer,
    buffers: &Buffers,
) -> Result<DescriptorRes> {
    let set_count = storage_imgs.len() as u32;
    let acc_count = acc_images.len() as u32;
    // assert_eq!(set_count, acc_count);

    let pool_sizes = [
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(set_count)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(acc_count)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(6)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.textures.len() as _)
            .build(),
    ];

    // println!("Image array size:{}", model.images.len());
    let pool = context.create_descriptor_pool(set_count + 1, &pool_sizes)?;

    let static_set = pool.allocate_set(&pipeline_res.static_dsl)?;
    let dynamic_sets = pool.allocate_sets(&pipeline_res.dynamic_dsl, set_count)?;

    static_set.update(&[
        WriteDescriptorSet {
            binding: VERTEX_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: if let Some((_, ani)) = &buffers.animation_buffers {
                    ani
                } else {
                    &buffers.vertex_buffer
                },
            },
        },
        WriteDescriptorSet {
            binding: INDEX_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.index_buffer,
            },
        },
    ]);
    static_set.update(&[
        WriteDescriptorSet {
            binding: GEO_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.geo_buffer,
            },
        },
        WriteDescriptorSet {
            binding: MAT_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.material_buffer,
            },
        },
    ]);
    static_set.update(&[
        WriteDescriptorSet {
            binding: AS_BIND,
            kind: WriteDescriptorSetKind::AccelerationStructure {
                acceleration_structure: &top_as.inner,
            },
        },
        WriteDescriptorSet {
            binding: UNIFORM_BIND,
            kind: WriteDescriptorSetKind::UniformBuffer { buffer: ubo_buffer },
        },
        WriteDescriptorSet {
            binding: DLIGHT_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.dlights_buffer,
            },
        },
        WriteDescriptorSet {
            binding: PLIGHT_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.plights_buffer,
            },
        },
    ]);

    let mut writes = vec![];
    for [_texture_index, image_index, sampler_index] in model.textures.iter() {
        let view = &model.views[*image_index];
        let sampler = &model.samplers[*sampler_index];
        writes.push(WriteDescriptorSet {
            binding: TEXTURE_BIND,
            kind: WriteDescriptorSetKind::CombinedImageSampler {
                view,
                sampler,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        });
    }
    static_set.update_texture_array(&writes);

    dynamic_sets.iter().enumerate().for_each(|(index, set)| {
        set.update(&[
            WriteDescriptorSet {
                binding: STORAGE_BIND,
                kind: WriteDescriptorSetKind::StorageImage {
                    layout: vk::ImageLayout::GENERAL,
                    view: &storage_imgs[index].view,
                },
            },
            WriteDescriptorSet {
                binding: ACC_BIND,
                kind: WriteDescriptorSetKind::StorageImage {
                    layout: vk::ImageLayout::GENERAL,
                    view: &acc_images[0].view,
                },
            },
        ]);
    });

    Ok(DescriptorRes {
        _pool: pool,
        dynamic_sets,
        static_set,
    })
}
