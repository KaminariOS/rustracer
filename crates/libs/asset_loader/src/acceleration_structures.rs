use crate::scene_graph::Doc;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};
use vulkan::{AccelerationStructure, Buffer, Context, Image, ImageBarrier, ImageView, Sampler};
use vulkan::ash::vk;
use vulkan::utils::create_gpu_only_buffer_from_data;
use crate::geometry::{GeoBuilder, Mesh, PrimInfo, Primitive, Vertex};
use anyhow::Result;
use vulkan::ash::vk::Packed24_8;
use vulkan::gpu_allocator::MemoryLocation;
use crate::{geometry, MaterialID};
use crate::material::MaterialRaw;

// One primitive per BLAS

struct Buffers {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
}

impl Buffers {
    fn new(geo_builder: &GeoBuilder) -> Result<Self>{
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
        Ok(Self {
            vertex_buffer,
            index_buffer
        })
    }
}

// One node per instance
struct VkInstance {
    pub(crate) transform_buffer: Buffer,
}

struct VkGlobal {
    pub(crate) _images: Vec<Image>,
    pub(crate) views: Vec<ImageView>,
    pub(crate) samplers: Vec<Sampler>,
    pub(crate) textures: Vec<[usize; 3]>,
    pub prim_info: Vec<PrimInfo>,
    materials: Vec<MaterialRaw>
}

fn primitive_to_vk_geometry(context: &Context, buffers: &Buffers, geo_builder: &GeoBuilder, geo_id: u32) -> BlasInput {
    let vertex_buffer_addr = buffers.vertex_buffer.get_device_address();
    let index_buffer_addr = buffers.index_buffer.get_device_address();
    let [v_len, i_len] = geo_builder.len[geo_id as usize];
    let [v_offset, i_offset, mat] = geo_builder.offsets[geo_id as usize];
    assert_eq!(i_len % 3, 0);
    let primitive_count = (i_len / 3) as u32;
    let as_geo_triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer_addr,
        })
        .vertex_stride(size_of::<Vertex>() as _)
        .max_vertex(v_len as _)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer_addr,
        })
        // .transform_data(vk::DeviceOrHostAddressConstKHR {
        //     device_address: transform_buffer_addr,
        // })
        .build();

    let as_geo_triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer_addr,
        })
        .vertex_stride(size_of::<Vertex>() as _)
        .max_vertex(v_len as _)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer_addr,
        })
        // .transform_data(vk::DeviceOrHostAddressConstKHR {
        //     device_address: transform_buffer_addr,
        // })
        .build();

    let geometry = vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
        .flags(vk::GeometryFlagsKHR::OPAQUE)
        .geometry(vk::AccelerationStructureGeometryDataKHR {
            triangles: as_geo_triangles_data,
        })
        .build();

    let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
        .first_vertex(v_offset)
        .primitive_count(primitive_count)
        .primitive_offset(i_offset * size_of::<u32>() as u32)
        // .transform_offset((node_index * size_of::<vk::TransformMatrixKHR>()) as u32)
        .build();
    BlasInput {
        geometries: vec![geometry],
        build_range_infos: vec![build_range_info],
        max_primitives: vec![primitive_count],
        geo_id: geo_id as _,
    }
}

pub fn create_model(context: &Context, doc: &Doc) -> (VkGlobal, Vec<AccelerationStructure>, AccelerationStructure) {
    let buffers = Buffers::new(&doc.geo_builder).unwrap();
    let mut blas_inputs: Vec<_> = doc.meshes.iter()
        .map(|m| m.primitives.iter())
        .flatten()
        .map(|p| primitive_to_vk_geometry(context, &buffers, &doc.geo_builder, p.geometry_id))
        .collect();
    blas_inputs.sort_by_key(|b| b.geo_id);

    let blases: Vec<_> = blas_inputs.into_iter().map(|b| context.create_bottom_level_acceleration_structure(
        &b.geometries,
        &b.build_range_infos,
        &b.max_primitives,
    ).unwrap()).collect();
    let tlas = create_top_as(context, doc, &blases);



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

    let mut textures = doc
        .textures
        .iter()
        .map(|t| [t.texture_index, t.image_index, t.sampler_index])
        .collect::<Vec<_>>();

    // Dummy texture
    if textures.is_empty() {
        textures.push([0; 3]);
    }

    (VkGlobal {
        _images: images,
        views,
        samplers,
        textures,
        prim_info: doc.geo_builder.flatten(),
        materials: doc.get_materials_raw()
    },
        blases,
        tlas)
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


fn create_top_as(context: &Context, doc: &Doc, blases: &Vec<AccelerationStructure>) -> AccelerationStructure {
    // Todo recursive
    // let tlases: Vec<_> = doc.get_current_scene().root_nodes.iter().map(|root| doc.nodes.get
    //     {
    //         let transform = node.get_world_transform();
    //         if let Some()
    //     }
    // ).collect();
     let mut ins = vec![];
     for (id, node) in doc.nodes.iter()
        .filter(|(_, node)| node.mesh.is_some()) {
         let transform = node.get_world_transform().to_cols_array();
         let mut matrix = [0.; 12];
         matrix.copy_from_slice(&transform[..12]);
         let mesh = &doc.meshes[node.mesh.unwrap()];
         let transform_matrix = vk::TransformMatrixKHR {matrix};
         let instances = mesh.primitives.iter().map(|p| {
             let geo_id = primitive.geometry_id;
             vk::AccelerationStructureInstanceKHR {
                 transform: transform_matrix,
                 instance_custom_index_and_mask: Packed24_8::new(geo_id, 0xFF),
                 instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(
                     0,
                     vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                 ),
                 acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                     device_handle: blases[geo_id as usize].address,
                 },
             }
         });
         ins.extend(instances);
     }
    let instance_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &ins,
    )?;
    let instance_buffer_addr = instance_buffer.get_device_address();

    let as_struct_geo = vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::INSTANCES)
        .flags(vk::GeometryFlagsKHR::OPAQUE)
        .geometry(vk::AccelerationStructureGeometryDataKHR {
            instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                .array_of_pointers(false)
                .data(vk::DeviceOrHostAddressConstKHR {
                    device_address: instance_buffer_addr,
                })
                .build(),
        })
        .build();

    let as_ranges = vk::AccelerationStructureBuildRangeInfoKHR::builder()
        .first_vertex(0)
        .primitive_count(1)
        .primitive_offset(0)
        .transform_offset(0)
        .build();


        context.create_top_level_acceleration_structure(&[as_struct_geo], &[as_ranges], &[1]).unwrap()

}

struct BlasInput {
    geometries: Vec<AccelerationStructureGeometryKHR>,
    build_range_infos: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    max_primitives: Vec<u32>,
    geo_id: u32
}
