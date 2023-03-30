use crate::{GeometryInfo, Model};
use app::vulkan::ash::vk;
use app::vulkan::{AccelerationStructure, Buffer, Context};
use std::mem::size_of;

use app::anyhow::Result;
use app::types::Mat4;
use app::vulkan::ash::vk::Packed24_8;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use gltf::Vertex;

pub struct BottomAS {
    pub inner: AccelerationStructure,
    pub geometry_info_buffer: Buffer,
}

pub struct TopAS {
    pub inner: AccelerationStructure,
    pub _instance_buffer: Buffer,
}

pub fn create_bottom_as(context: &mut Context, model: &Model) -> Result<BottomAS> {
    let vertex_buffer_addr = model.vertex_buffer.get_device_address();

    let index_buffer_addr = model.index_buffer.get_device_address();

    let transform_buffer_addr = model.transform_buffer.get_device_address();

    let as_geo_triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer_addr,
        })
        .vertex_stride(size_of::<Vertex>() as _)
        .max_vertex(model.gltf.vertices.len() as _)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer_addr,
        })
        .transform_data(vk::DeviceOrHostAddressConstKHR {
            device_address: transform_buffer_addr,
        })
        .build();

    let mut geometry_infos = vec![];
    let mut as_geometries = vec![];
    let mut as_ranges = vec![];
    let mut max_primitive_counts = vec![];

    for (node_index, node) in model.gltf.nodes.iter().enumerate() {
        let mesh = &node.mesh;
        // println!("{:?}", mesh.material.emissive_factor);
        let primitive_count = (mesh.index_count / 3) as u32;
        // mesh.material
        geometry_infos.push(GeometryInfo {
            transform: Mat4::from_iterator(node.transform.iter().flatten().map(|x| *x)),
            base_color: mesh.material.base_color,
            base_color_texture: mesh.material.base_color_texture,
            normal_texture: mesh.material.normal_texture,
            metallic_roughness_texture: mesh.material.metallic_roughness_texture,
            metallic_factor: mesh.material.metallic_factor,
            roughness: mesh.material.roughness,
            ior: mesh.material.ior,
            _padding: 0.0,
            vertex_offset: mesh.vertex_offset,
            index_offset: mesh.index_offset,
            emissive_factor: mesh.material.emissive_factor,
        });

        as_geometries.push(
            vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: as_geo_triangles_data,
                })
                .build(),
        );

        as_ranges.push(
            vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(mesh.vertex_offset)
                .primitive_count(primitive_count)
                .primitive_offset(mesh.index_offset * size_of::<u32>() as u32)
                .transform_offset((node_index * size_of::<vk::TransformMatrixKHR>()) as u32)
                .build(),
        );

        max_primitive_counts.push(primitive_count)
    }

    let geometry_info_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        &geometry_infos,
    )?;

    let inner = context.create_bottom_level_acceleration_structure(
        &as_geometries,
        &as_ranges,
        &max_primitive_counts,
    )?;

    Ok(BottomAS {
        inner,
        geometry_info_buffer,
    })
}

pub fn create_top_as(context: &mut Context, bottom_as: &BottomAS) -> Result<TopAS> {
    #[rustfmt::skip]
        let transform_matrix = vk::TransformMatrixKHR { matrix: [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0
    ]};

    let as_instance = vk::AccelerationStructureInstanceKHR {
        transform: transform_matrix,
        instance_custom_index_and_mask: Packed24_8::new(0, 0xFF),
        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(
            0,
            vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
        ),
        acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
            device_handle: bottom_as.inner.address,
        },
    };

    let instance_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &[as_instance],
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

    let inner =
        context.create_top_level_acceleration_structure(&[as_struct_geo], &[as_ranges], &[1])?;

    Ok(TopAS {
        inner,
        _instance_buffer: instance_buffer,
    })
}
