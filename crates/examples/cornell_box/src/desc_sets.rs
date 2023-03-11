use app::vulkan::ash::vk;
use app::vulkan::{Buffer, Context, DescriptorPool, DescriptorSet, WriteDescriptorSet, WriteDescriptorSetKind};
use app::anyhow::Result;
use app::ImageAndView;
use crate::ACC_BIND;
use crate::acceleration_structure::{BottomAS, TopAS};
use crate::model::Model;
use crate::pipeline_res::PipelineRes;

pub struct DescriptorRes {
    _pool: DescriptorPool,
    pub(crate) static_set: DescriptorSet,
    pub(crate) dynamic_sets: Vec<DescriptorSet>,
}

pub fn create_descriptor_sets(
    context: &Context,
    pipeline_res: &PipelineRes,
    model: &Model,
    bottom_as: &BottomAS,
    top_as: &TopAS,
    storage_imgs: &[ImageAndView],
    acc_images: &[ImageAndView],
    ubo_buffer: &Buffer,
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
            .descriptor_count(3)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.images.len() as _)
            .build(),
    ];

    let pool = context.create_descriptor_pool(set_count + 1, &pool_sizes)?;

    let static_set = pool.allocate_set(&pipeline_res.static_dsl)?;
    let dynamic_sets = pool.allocate_sets(&pipeline_res.dynamic_dsl, set_count)?;

    static_set.update(&[
        WriteDescriptorSet {
            binding: 0,
            kind: WriteDescriptorSetKind::AccelerationStructure {
                acceleration_structure: &top_as.inner,
            },
        },
        WriteDescriptorSet {
            binding: 2,
            kind: WriteDescriptorSetKind::UniformBuffer { buffer: ubo_buffer },
        },
        WriteDescriptorSet {
            binding: 3,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &model.vertex_buffer,
            },
        },
        WriteDescriptorSet {
            binding: 4,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &model.index_buffer,
            },
        },
        WriteDescriptorSet {
            binding: 5,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &bottom_as.geometry_info_buffer,
            },
        },
    ]);

    for (image_index, sampler_index) in model.textures.iter() {
        let view = &model.views[*image_index];
        let sampler = &model.samplers[*sampler_index];

        static_set.update(&[
            WriteDescriptorSet {
                binding: 6,
                kind: WriteDescriptorSetKind::CombinedImageSampler {
                    view,
                    sampler,
                    layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                },
            }
        ]);
    }

    dynamic_sets.iter().enumerate().for_each(|(index, set)| {
        set.update(&[
            WriteDescriptorSet {
                binding: 1,
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
            }
        ]);
    });

    Ok(DescriptorRes {
        _pool: pool,
        dynamic_sets,
        static_set,
    })
}
