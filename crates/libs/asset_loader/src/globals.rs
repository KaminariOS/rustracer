use crate::geometry::{GeoBuilder, PrimInfo};
use crate::material::MaterialRaw;
use crate::scene_graph::Doc;
use crate::texture;
use anyhow::Result;
use std::mem::size_of_val;
use vulkan::ash::vk;
use vulkan::gpu_allocator::MemoryLocation;
use vulkan::utils::create_gpu_only_buffer_from_data;
use vulkan::{Buffer, BufferBarrier, Context, Image, ImageBarrier, ImageView, Sampler};
use crate::light::LightRaw;


pub struct Buffers {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub geo_buffer: Buffer,
    pub material_buffer: Buffer,
    pub dlights_buffer: Buffer,
    pub plights_buffer: Buffer,
}

impl Buffers {
    pub fn new(context: &Context, geo_builder: &GeoBuilder, globals: &VkGlobal) -> Result<Self> {
        let vertices = geo_builder.vertices.as_slice();
        let indices = geo_builder.indices.as_slice();

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
        let geo_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.prim_info,
        )?;

        let material_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.materials,
        )?;

        let dlights_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.d_lights,
        )?;

        let plights_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.d_lights,
        )?;

        // println!("v_b: {:#02x}, i_b: {:#02x}, g_b: {:#02x}, m_b: {:#02x}", vertex_buffer.as_raw(), index_buffer.as_raw(), geo_buffer.as_raw(), material_buffer.as_raw());
        // let src_stage = vk::PipelineStageFlags2::TRANSFER | vk::PipelineStageFlags2::ALL_COMMANDS;
        // let ray_tracing_dst = vk::PipelineStageFlags2::ALL_COMMANDS;
        // let as_build = vk::PipelineStageFlags2::ALL_COMMANDS;
        // let src_access = vk::AccessFlags2::TRANSFER_WRITE;
        // let des_access= vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::HOST_WRITE | vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::HOST_READ;
        // context.execute_one_time_commands(|cmd| cmd.pipeline_buffer_barriers(
        //     &[
        //         BufferBarrier {
        //             buffer: &vertex_buffer,
        //             src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        //             dst_access_mask: des_access,
        //             src_stage_mask: src_stage,
        //             dst_stage_mask: as_build
        //         },
        //         BufferBarrier {
        //             buffer: &index_buffer,
        //             src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        //             dst_access_mask: des_access,
        //             src_stage_mask: src_stage,
        //             dst_stage_mask: as_build
        //         },
        //         BufferBarrier {
        //             buffer: &geo_buffer,
        //             src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        //             dst_access_mask: des_access,
        //             src_stage_mask: src_stage,
        //             dst_stage_mask: ray_tracing_dst
        //         },
        //         BufferBarrier {
        //             buffer: &material_buffer,
        //             src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        //             dst_access_mask: des_access,
        //             src_stage_mask: src_stage,
        //             dst_stage_mask: ray_tracing_dst
        //         },
        //     BufferBarrier {
        //         buffer: &lights_buffer,
        //         src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        //         dst_access_mask: des_access,
        //         src_stage_mask: src_stage,
        //         dst_stage_mask: ray_tracing_dst
        //     }
        //     ]
        // ))?;
        Ok(Self {
            vertex_buffer,
            index_buffer,
            geo_buffer,
            material_buffer,
            plights_buffer,
            dlights_buffer,
        })
    }
}

pub struct VkGlobal {
    pub(crate) _images: Vec<Image>,
    pub views: Vec<ImageView>,
    pub samplers: Vec<Sampler>,
    pub textures: Vec<[usize; 3]>,

    pub prim_info: Vec<PrimInfo>,
    materials: Vec<MaterialRaw>,
    d_lights: Vec<LightRaw>,
    p_lights: Vec<LightRaw>,
}
pub fn create_global(context: &Context, doc: &Doc) -> Result<VkGlobal> {
    let mut images = vec![];
    let mut views = vec![];

    doc.images.iter().try_for_each::<_, Result<_>>(|i| {
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


    let samplers = doc
        .samplers
        .iter()
        .map(|s| {
            let sampler_info = map_gltf_sampler(s);
            context.create_sampler(&sampler_info)
        })
        .collect::<Result<Vec<_>>>()?;


    let textures = doc
        .textures
        .iter()
        .map(|t| [t.index, t.image_index, t.sampler_index])
        .collect::<Vec<_>>();
    // Dummy texture
    let [d_lights, p_lights] = doc.get_lights_raw();

    Ok(VkGlobal {
        _images: images,
        views,
        samplers,
        textures,
        prim_info: doc.geo_builder.flatten(),
        materials: doc.get_materials_raw(),
        d_lights,
        p_lights
    })
}

fn map_gltf_sampler<'a>(sampler: &texture::Sampler) -> vk::SamplerCreateInfoBuilder<'a> {
    let mag_filter = match sampler.mag_filter {
        texture::MagFilter::Linear => vk::Filter::LINEAR,
        texture::MagFilter::Nearest => vk::Filter::NEAREST,
    };

    let min_filter = match sampler.min_filter {
        texture::MinFilter::Linear
        | texture::MinFilter::LinearMipmapLinear
        | texture::MinFilter::LinearMipmapNearest => vk::Filter::LINEAR,
        texture::MinFilter::Nearest
        | texture::MinFilter::NearestMipmapLinear
        | texture::MinFilter::NearestMipmapNearest => vk::Filter::NEAREST,
    };

    vk::SamplerCreateInfo::builder()
        .mag_filter(mag_filter)
        .min_filter(min_filter)
}
