use crate::geometry::{GeoBuilder, Vertex};
use crate::globals::Buffers;

use crate::scene_graph::{Doc, Node};

use anyhow::Result;

use glam::Mat4;
use log::info;
use std::mem::size_of;
use std::time::Instant;

use vulkan::ash::vk;
use vulkan::ash::vk::Packed24_8;

use vulkan::utils::create_gpu_only_buffer_from_data_batch;
use vulkan::{AccelerationStructure, Buffer, Context};

fn primitive_to_vk_geometry(
    _context: &Context,
    buffers: &Buffers,
    geo_builder: &GeoBuilder,
    geo_id: u32,
) -> BlasInput {
    let vertex_buffer = if let Some(ani) = &buffers.animation_buffers {
        &ani.1
    } else {
        &buffers.vertex_buffer
    };
    let vertex_buffer_addr = vertex_buffer.get_device_address();
    let index_buffer_addr = buffers.index_buffer.get_device_address();
    let [v_len, i_len] = geo_builder.len[geo_id as usize];
    let [v_offset, i_offset, _mat] = geo_builder.offsets[geo_id as usize];
    let is_opaque = geo_builder.is_opaque(geo_id);
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
        .flags(if is_opaque {
            vk::GeometryFlagsKHR::OPAQUE
        } else {
            vk::GeometryFlagsKHR::empty()
        })
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
        as_geo_data: as_geo_triangles_data,
        geometries: vec![geometry],
        build_range_infos: vec![build_range_info],
        max_primitives: vec![primitive_count],
        geo_id: geo_id as _,
    }
}
// https://developer.nvidia.com/blog/best-practices-for-using-nvidia-rtx-ray-tracing-updated/
// For TLAS, consider the PREFER_FAST_TRACE flag and perform only rebuilds. Often, this results in best overall performance. The rationale is that making the TLAS as high quality as possible regardless of the movement occurring in the scene is important and doesn’t cost too much.
pub fn create_as(
    context: &Context,
    doc: &Doc,
    buffers: &Buffers,
    flags: vk::BuildAccelerationStructureFlagsKHR,
) -> Result<(Vec<AccelerationStructure>, TopAS)> {
    let time = Instant::now();
    let mut blas_inputs: Vec<_> = doc
        .meshes
        .iter()
        .map(|m| m.primitives.iter())
        .flatten()
        .map(|p| primitive_to_vk_geometry(context, &buffers, &doc.geo_builder, p.geometry_id))
        .collect();
    blas_inputs.sort_by_key(|b| b.geo_id);
    let cmd_buffer = context
        .command_pool
        .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)?;
    cmd_buffer.begin(Some(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT))?;

    let (blases, _s): (Vec<_>, Vec<_>) = blas_inputs
        .iter()
        .map(|b| {
            context
                .create_bottom_level_acceleration_structure_batch(
                    &b.geometries,
                    &b.build_range_infos,
                    &b.max_primitives,
                    // vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
                    flags,
                    &cmd_buffer,
                )
                .unwrap()
        })
        .unzip();

    unsafe {
        context.device.inner.cmd_pipeline_barrier2(
            cmd_buffer.inner,
            &vk::DependencyInfo::builder()
                .memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR)
                    .dst_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR)
                    .src_stage_mask(vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
                    .dst_stage_mask(vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
                    .build()])
                .build(),
        );
    }
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
    let tlas = create_top_as(context, doc, &blases, flags)?;
    // info!(
    //     "Finish building acceleration structure: {}s",
    //     time.elapsed().as_secs()
    // );
    Ok((blases, tlas))
}

pub struct TopAS {
    pub inner: AccelerationStructure,
    pub _instance_buffer: Buffer,
}

pub fn create_top_as(
    context: &Context,
    doc: &Doc,
    blases: &Vec<AccelerationStructure>,
    flags: vk::BuildAccelerationStructureFlagsKHR,
) -> Result<TopAS> {
    let mut ins = vec![];
    let mut f = |node: &Node| {
        // Row major.
        let transform = if node.skin.is_none() {
            node.get_world_transform().transpose().to_cols_array()
        } else {
            Mat4::IDENTITY.to_cols_array()
        };
        let mut matrix = [0.; 12];
        matrix.copy_from_slice(&transform[..12]);
        if let Some(mesh) = node.mesh.map(|m| &doc.meshes[m]) {
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
    };
    doc.traverse_root_nodes(&mut f);

    let cmd_buffer = context
        .command_pool
        .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)?;
    cmd_buffer.begin(Some(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT))?;

    let (instance_buffer, _i) = create_gpu_only_buffer_from_data_batch(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &ins,
        &cmd_buffer,
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

    unsafe {
        context.device.inner.cmd_pipeline_barrier2(
            cmd_buffer.inner,
            &vk::DependencyInfo::builder()
                .memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR)
                    .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .dst_stage_mask(vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR)
                    .build()])
                .build(),
        );
    }
    let (inner, _s) = context.create_top_level_acceleration_structure_batch(
        &[as_struct_geo],
        &[as_ranges],
        &[ins.len() as u32],
        flags,
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
    // // Free
    context.command_pool.free_command_buffer(&cmd_buffer)?;

    Ok(TopAS {
        inner,
        _instance_buffer: instance_buffer,
    })
}

pub struct BlasInput {
    as_geo_data: vk::AccelerationStructureGeometryTrianglesDataKHR,
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    build_range_infos: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    max_primitives: Vec<u32>,
    geo_id: u32,
}
