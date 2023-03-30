use crate::gui_state::Scene;
use app::anyhow::Result;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{Buffer, Context, Image, ImageBarrier, ImageView, Sampler};
use std::mem::size_of_val;

pub struct Model {
    pub(crate) gltf: gltf::Model,
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) transform_buffer: Buffer,
    pub(crate) _images: Vec<Image>,
    pub(crate) views: Vec<ImageView>,
    pub(crate) samplers: Vec<Sampler>,
    pub(crate) textures: Vec<[usize; 3]>,
}

pub fn create_model(context: &Context, scene: Scene) -> Result<Model> {
    let model = gltf::load_file(scene.path())?;
    let vertices = model.vertices.as_slice();
    let indices = model.indices.as_slice();

    let vertex_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
            | vk::BufferUsageFlags::STORAGE_BUFFER,
        vertices,
    )?;

    let index_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
            | vk::BufferUsageFlags::STORAGE_BUFFER,
        indices,
    )?;

    let transforms = model
        .nodes
        .iter()
        .map(|n| {
            let transform = n.transform;
            let r0 = transform[0];
            let r1 = transform[1];
            let r2 = transform[2];
            let r3 = transform[3];

            #[rustfmt::skip]
                let matrix = [
                r0[0], r1[0], r2[0], r3[0],
                r0[1], r1[1], r2[1], r3[1],
                r0[2], r1[2], r2[2], r3[2],
            ];

            vk::TransformMatrixKHR { matrix }
        })
        .collect::<Vec<_>>();
    let transform_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &transforms,
    )?;

    let mut images = vec![];
    let mut views = vec![];

    model.images.iter().try_for_each::<_, Result<_>>(|i| {
        let width = i.width;
        let height = i.height;
        let pixels = i.pixels.as_slice();

        let staging = context.create_buffer(
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            size_of_val(pixels) as _,
        )?;

        staging.copy_data_to_buffer(pixels)?;

        let image = context.create_image(
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_SRGB,
            width,
            height,
        )?;

        context.execute_one_time_commands(|cmd| {
            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            }]);

            cmd.copy_buffer_to_image(&staging, &image, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        let view = image.create_image_view()?;

        images.push(image);
        views.push(view);

        Ok(())
    })?;

    // Dummy textures
    if images.is_empty() {
        let image = context.create_image(
            vk::ImageUsageFlags::SAMPLED,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_SRGB,
            1,
            1,
        )?;

        context.execute_one_time_commands(|cmd| {
            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        let view = image.create_image_view()?;

        images.push(image);
        views.push(view);
    }

    let mut samplers = model
        .samplers
        .iter()
        .map(|s| {
            let sampler_info = map_gltf_sampler(s);
            context.create_sampler(&sampler_info)
        })
        .collect::<Result<Vec<_>>>()?;

    // Dummy sampler
    if samplers.is_empty() {
        let sampler_info = vk::SamplerCreateInfo::builder();
        let sampler = context.create_sampler(&sampler_info)?;
        samplers.push(sampler);
    }

    let mut textures = model
        .textures
        .iter()
        .map(|t| [t.texture_index, t.image_index, t.sampler_index])
        .collect::<Vec<_>>();

    // Dummy texture
    if textures.is_empty() {
        textures.push([0; 3]);
    }

    Ok(Model {
        gltf: model,
        vertex_buffer,
        index_buffer,
        transform_buffer,
        _images: images,
        views,
        samplers,
        textures,
    })
}

fn map_gltf_sampler<'a>(sampler: &gltf::Sampler) -> vk::SamplerCreateInfoBuilder<'a> {
    let mag_filter = match sampler.mag_filter {
        gltf::MagFilter::Linear => vk::Filter::LINEAR,
        gltf::MagFilter::Nearest => vk::Filter::NEAREST,
    };

    let min_filter = match sampler.min_filter {
        gltf::MinFilter::Linear
        | gltf::MinFilter::LinearMipmapLinear
        | gltf::MinFilter::LinearMipmapNearest => vk::Filter::LINEAR,
        gltf::MinFilter::Nearest
        | gltf::MinFilter::NearestMipmapLinear
        | gltf::MinFilter::NearestMipmapNearest => vk::Filter::NEAREST,
    };

    vk::SamplerCreateInfo::builder()
        .mag_filter(mag_filter)
        .min_filter(min_filter)
}
