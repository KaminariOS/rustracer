use crate::cubumap::SkyBox;
use crate::geometry::{GeoBuilder, PrimInfo};
use crate::image::TexGamma;
use crate::light::LightRaw;
use crate::material::MaterialRaw;
use crate::scene_graph::Doc;
use crate::texture;
use crate::texture::WrapMode;
use anyhow::Result;
use log::info;
use std::mem::size_of_val;
use std::time::Instant;

use crate::skinning::{JointRaw, MAX_JOINTS};
use vulkan::ash::vk;
use vulkan::ash::vk::SamplerAddressMode;
use vulkan::gpu_allocator::MemoryLocation;
use vulkan::utils::create_gpu_only_buffer_from_data_batch;
use vulkan::{
    Buffer, Context, DescriptorSet, Image, ImageBarrier, ImageView, Sampler, WriteDescriptorSet,
    WriteDescriptorSetKind,
};

impl Into<vk::Format> for TexGamma {
    fn into(self) -> vk::Format {
        match self {
            TexGamma::Linear => vk::Format::R8G8B8A8_UNORM,
            TexGamma::Srgb => vk::Format::R8G8B8A8_SRGB,
        }
    }
}

pub struct Buffers {
    pub vertex_buffer: Buffer,
    pub animation_buffers: Option<(Buffer, Buffer)>,
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
        let no_skin = globals.skins.is_empty();
        let now = Instant::now();
        let cmd_buffer = context
            .command_pool
            .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)?;
        cmd_buffer.begin(Some(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT))?;

        let (vertex_buffer, _v) = create_gpu_only_buffer_from_data_batch(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            vertices,
            &cmd_buffer,
        )?;
        let animation_buffer = if !no_skin {
            Some(create_gpu_only_buffer_from_data_batch(
                context,
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
                vertices,
                &cmd_buffer,
            )?)
        } else {
            None
        };

        let (index_buffer, _i) = create_gpu_only_buffer_from_data_batch(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            indices,
            &cmd_buffer,
        )?;
        let (geo_buffer, _g) = create_gpu_only_buffer_from_data_batch(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.prim_info,
            &cmd_buffer,
        )?;

        let (material_buffer, _s) = create_gpu_only_buffer_from_data_batch(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            &globals.materials,
            &cmd_buffer,
        )?;

        // End recording
        cmd_buffer.end()?;

        // Submit and wait
        let fence = context.create_fence(None)?;
        // let fence = Fence::null(&context.device);
        context
            .graphics_queue
            .submit(&cmd_buffer, None, None, &fence)?;
        fence.wait(None)?;
        // Free
        context.command_pool.free_command_buffer(&cmd_buffer)?;
        let animation_buffer = animation_buffer.map(|(b, _)| b);

        let _size_of_slice = size_of_val(globals.d_lights.as_slice());
        let _size = size_of_val(&globals.d_lights);
        let dlights_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of_val(globals.d_lights.as_slice()) as _,
        )?;
        dlights_buffer.copy_data_to_buffer(globals.d_lights.as_slice())?;

        let plights_buffer = context.create_buffer(
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of_val(globals.p_lights.as_slice()) as _,
        )?;
        plights_buffer.copy_data_to_buffer(globals.p_lights.as_slice())?;

        let animation_buffers = if let Some(ani) = animation_buffer {
            let skins_buffer = context.create_buffer(
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::CpuToGpu,
                size_of_val(globals.skins.as_slice()) as _,
            )?;
            skins_buffer.copy_data_to_buffer(globals.skins.as_slice())?;
            Some((skins_buffer, ani))
        } else {
            None
        };

        info!("Buffers: {}s", now.elapsed().as_secs());
        // println!("v_b: {:#02x}, i_b: {:#02x}, g_b: {:#02x}, m_b: {:#02x}", vertex_buffer.as_raw(), index_buffer.as_raw(), geo_buffer.as_raw(), material_buffer.as_raw());
        let _src_stage = vk::PipelineStageFlags2::TRANSFER | vk::PipelineStageFlags2::ALL_COMMANDS;
        let _ray_tracing_dst = vk::PipelineStageFlags2::ALL_COMMANDS;
        Ok(Self {
            vertex_buffer,
            animation_buffers,
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
    pub d_lights: Vec<LightRaw>,
    pub p_lights: Vec<LightRaw>,
    pub skins: Vec<[JointRaw; MAX_JOINTS]>,
}

pub struct SkyboxResource {
    // skybox: SkyBox,
    pub image: Image,
    pub view: ImageView,
    pub sampler: Sampler,
}

impl SkyboxResource {
    pub fn new(context: &Context, path: &str) -> Result<Self> {
        let skybox = SkyBox::new(path)?;
        let (image, view) = create_cubemap_view(context, &skybox)?;
        let sampler = context.create_sampler(&map_gltf_sampler(&skybox.sampler))?;
        Ok(Self {
            // skybox,
            image,
            view,
            sampler,
        })
    }

    pub fn update_desc(&self, desc: &DescriptorSet, binding: u32) {
        let skybox_write = [WriteDescriptorSet {
            binding,
            kind: WriteDescriptorSetKind::CombinedImageSampler {
                view: &self.view,
                sampler: &self.sampler,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        }];
        desc.update_texture_array(&skybox_write);
    }
}

pub fn create_cubemap_view(context: &Context, skybox: &SkyBox) -> Result<(Image, ImageView)> {
    let [w, h] = skybox.get_extents();
    info!("Skybox: [w: {}, h: {}]", w, h);
    let pixels_ref = skybox.collector.as_slice();

    let staging = context.create_buffer(
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
        size_of_val(pixels_ref) as _,
    )?;

    staging.copy_data_to_buffer(pixels_ref)?;

    let image = context.create_cubemap_image(
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        MemoryLocation::GpuOnly,
        skybox.get_gamma().into(),
        w,
        h,
    )?;

    const FACES: u32 = 6;
    context.execute_one_time_commands(|cmd| {
        cmd.pipeline_image_barriers_layers(
            &[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            }],
            FACES,
        );

        cmd.copy_buffer_to_image_layer(
            &staging,
            &image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            FACES,
        );

        cmd.pipeline_image_barriers_layers(
            &[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }],
            FACES,
        );
    })?;
    let view = image.create_cubemap_view()?;
    Ok((image, view))
}

fn create_image_view(context: &Context, i: &crate::image::Image) -> Result<(Image, ImageView)> {
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
        i.gamma.into(),
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
    Ok((image, view))
}

pub fn create_global(context: &Context, doc: &Doc) -> Result<VkGlobal> {
    info!("Fully opaque: {}", doc.geo_builder.fully_opaque());
    let mut images = vec![];
    let mut views = vec![];

    doc.images.iter().try_for_each::<_, Result<_>>(|i| {
        let (image, view) = create_image_view(context, i)?;
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
        p_lights,
        skins: doc.get_skins(),
    })
}

impl Into<vk::SamplerAddressMode> for WrapMode {
    fn into(self) -> SamplerAddressMode {
        match self {
            WrapMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            WrapMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            WrapMode::Repeat => vk::SamplerAddressMode::REPEAT,
        }
    }
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
        .address_mode_u(sampler.wrap_s.into())
        .address_mode_v(sampler.wrap_t.into())
}
