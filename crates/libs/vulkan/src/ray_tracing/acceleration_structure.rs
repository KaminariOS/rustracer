use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::{Buffer, Context, RayTracingContext};

pub struct AccelerationStructure {
    ray_tracing: Arc<RayTracingContext>,
    pub(crate) inner: vk::AccelerationStructureKHR,
    _buffer: Buffer,
    pub address: u64,
}

impl AccelerationStructure {
    pub(crate) fn new(
        context: &Context,
        ray_tracing: Arc<RayTracingContext>,
        level: vk::AccelerationStructureTypeKHR,
        as_geometry: &[vk::AccelerationStructureGeometryKHR],
        as_ranges: &[vk::AccelerationStructureBuildRangeInfoKHR],
        max_primitive_counts: &[u32],
    ) -> Result<Self> {
        let build_geo_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(level)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(as_geometry);

        let build_size = unsafe {
            ray_tracing
                .acceleration_structure_fn
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_geo_info,
                    max_primitive_counts,
                )
        };

        let buffer = context.create_buffer(
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            MemoryLocation::GpuOnly,
            build_size.acceleration_structure_size,
        )?;

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(buffer.inner)
            .size(build_size.acceleration_structure_size)
            .ty(level);
        let inner = unsafe {
            ray_tracing
                .acceleration_structure_fn
                .create_acceleration_structure(&create_info, None)?
        };

        let scratch_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            MemoryLocation::GpuOnly,
            build_size.build_scratch_size,
        )?;
        let scratch_buffer_address = scratch_buffer.get_device_address();

        let build_geo_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(level)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(as_geometry)
            .dst_acceleration_structure(inner)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer_address,
            });

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.build_acceleration_structures(&build_geo_info, as_ranges);
        })?;

        let address_info =
            vk::AccelerationStructureDeviceAddressInfoKHR::builder().acceleration_structure(inner);
        let address = unsafe {
            ray_tracing
                .acceleration_structure_fn
                .get_acceleration_structure_device_address(&address_info)
        };

        Ok(Self {
            ray_tracing,
            inner,
            _buffer: buffer,
            address,
        })
    }
}

impl Context {
    pub fn create_bottom_level_acceleration_structure(
        &self,
        as_geometry: &[vk::AccelerationStructureGeometryKHR],
        as_ranges: &[vk::AccelerationStructureBuildRangeInfoKHR],
        max_primitive_counts: &[u32],
    ) -> Result<AccelerationStructure> {
        let ray_tracing = self.ray_tracing.clone().expect(
            "Cannot call Context::create_bottom_level_acceleration_structure when ray tracing is not enabled",
        );

        AccelerationStructure::new(
            self,
            ray_tracing,
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            as_geometry,
            as_ranges,
            max_primitive_counts,
        )
    }

    pub fn create_top_level_acceleration_structure(
        &self,
        as_geometry: &[vk::AccelerationStructureGeometryKHR],
        as_ranges: &[vk::AccelerationStructureBuildRangeInfoKHR],
        max_primitive_counts: &[u32],
    ) -> Result<AccelerationStructure> {
        let ray_tracing = self.ray_tracing.clone().expect(
            "Cannot call Context::create_top_level_acceleration_structure when ray tracing is not enabled",
        );

        AccelerationStructure::new(
            self,
            ray_tracing,
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            as_geometry,
            as_ranges,
            max_primitive_counts,
        )
    }
}

impl Drop for AccelerationStructure {
    fn drop(&mut self) {
        unsafe {
            self.ray_tracing
                .acceleration_structure_fn
                .destroy_acceleration_structure(self.inner, None);
        }
    }
}
