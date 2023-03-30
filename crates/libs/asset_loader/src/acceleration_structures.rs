use crate::geometry::{GeoBuilder, Mesh, PrimInfo, Primitive, Vertex};
use crate::globals::Buffers;
use crate::material::MaterialRaw;
use crate::scene_graph::Doc;
use crate::{geometry, MaterialID};
use anyhow::Result;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};
use vulkan::ash::vk;
use vulkan::ash::vk::Packed24_8;
use vulkan::gpu_allocator::MemoryLocation;
use vulkan::utils::create_gpu_only_buffer_from_data;
use vulkan::{AccelerationStructure, Buffer, Context, Image, ImageBarrier, ImageView, Sampler};

fn primitive_to_vk_geometry(
    context: &Context,
    buffers: &Buffers,
    geo_builder: &GeoBuilder,
    geo_id: u32,
) -> BlasInput {
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

pub fn create_as(
    context: &Context,
    doc: &Doc,
    buffers: &Buffers,
) -> Result<(Vec<AccelerationStructure>, TopAS)> {
    let mut blas_inputs: Vec<_> = doc
        .meshes
        .iter()
        .map(|m| m.primitives.iter())
        .flatten()
        .map(|p| primitive_to_vk_geometry(context, &buffers, &doc.geo_builder, p.geometry_id))
        .collect();
    blas_inputs.sort_by_key(|b| b.geo_id);

    let blases: Vec<_> = blas_inputs
        .into_iter()
        .map(|b| {
            context
                .create_bottom_level_acceleration_structure(
                    &b.geometries,
                    &b.build_range_infos,
                    &b.max_primitives,
                )
                .unwrap()
        })
        .collect();
    let tlas = create_top_as(context, doc, &blases)?;

    Ok((blases, tlas))
}

pub struct TopAS {
    pub inner: AccelerationStructure,
    pub _instance_buffer: Buffer,
}

fn create_top_as(
    context: &Context,
    doc: &Doc,
    blases: &Vec<AccelerationStructure>,
) -> Result<TopAS> {
    // Todo recursive
    // let tlases: Vec<_> = doc.get_current_scene().root_nodes.iter().map(|root| doc.nodes.get
    //     {
    //         let transform = node.get_world_transform();
    //         if let Some()
    //     }
    // ).collect();
    let mut ins = vec![];
    for (id, node) in doc.nodes.iter().filter(|(_, node)| node.mesh.is_some()) {
        let transform = node.get_world_transform().transpose().to_cols_array();
        let mut matrix = [0.; 12];
        // Row major.
        matrix.copy_from_slice(&transform[..12]);
        let mesh = &doc.meshes[node.mesh.unwrap()];
        let transform_matrix = vk::TransformMatrixKHR { matrix };
        let instances = mesh.primitives.iter().map(|p| {
            let geo_id = p.geometry_id;
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
        .primitive_count(ins.len() as u32)
        .primitive_offset(0)
        // .transform_offset(0)
        .build();

    Ok(TopAS {
        inner: context.create_top_level_acceleration_structure(
            &[as_struct_geo],
            &[as_ranges],
            &[ins.len() as u32],
        )?,
        _instance_buffer: instance_buffer,
    })
}

struct BlasInput {
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    build_range_infos: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    max_primitives: Vec<u32>,
    geo_id: u32,
}
