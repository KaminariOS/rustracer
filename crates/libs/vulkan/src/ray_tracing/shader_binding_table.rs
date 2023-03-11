use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::{utils::compute_aligned_size, Buffer, Context, RayTracingContext, RayTracingPipeline};

pub struct ShaderBindingTable {
    _buffer: Buffer,
    pub(crate) raygen_region: vk::StridedDeviceAddressRegionKHR,
    pub(crate) miss_region: vk::StridedDeviceAddressRegionKHR,
    pub(crate) hit_region: vk::StridedDeviceAddressRegionKHR,
}

impl ShaderBindingTable {
    pub(crate) fn new(
        context: &Context,
        ray_tracing: &RayTracingContext,
        pipeline: &RayTracingPipeline,
    ) -> Result<Self> {
        let desc = pipeline.shader_group_info;

        // Handle size & aligment
        let handle_size = ray_tracing.pipeline_properties.shader_group_handle_size;
        let handle_alignment = ray_tracing
            .pipeline_properties
            .shader_group_handle_alignment;
        let aligned_handle_size = compute_aligned_size(handle_size, handle_alignment);
        let handle_pad = aligned_handle_size - handle_size;

        let group_alignment = ray_tracing.pipeline_properties.shader_group_base_alignment;

        // Get Handles
        let data_size = desc.group_count * handle_size;
        let handles = unsafe {
            ray_tracing
                .pipeline_fn
                .get_ray_tracing_shader_group_handles(
                    pipeline.inner,
                    0,
                    desc.group_count,
                    data_size as _,
                )?
        };

        // Region sizes
        let raygen_region_size = compute_aligned_size(
            desc.raygen_shader_count * aligned_handle_size,
            group_alignment,
        );

        let miss_region_size = compute_aligned_size(
            desc.miss_shader_count * aligned_handle_size,
            group_alignment,
        );
        let hit_region_size =
            compute_aligned_size(desc.hit_shader_count * aligned_handle_size, group_alignment);

        // Create sbt data
        let buffer_size = raygen_region_size + miss_region_size + hit_region_size;
        let mut stb_data = Vec::<u8>::with_capacity(buffer_size as _);
        let groups_shader_count = [
            desc.raygen_shader_count,
            desc.miss_shader_count,
            desc.hit_shader_count,
        ];

        let mut offset = 0;
        // for each groups
        for group_shader_count in groups_shader_count {
            let group_size = group_shader_count * aligned_handle_size;
            let aligned_group_size = compute_aligned_size(group_size, group_alignment);
            let group_pad = aligned_group_size - group_size;

            // for each handle
            for _ in 0..group_shader_count {
                //copy handle
                for _ in 0..handle_size as usize {
                    stb_data.push(handles[offset]);
                    offset += 1;
                }

                // pad handle to alignment
                for _ in 0..handle_pad {
                    stb_data.push(0);
                }
            }

            // pad group to alignment
            for _ in 0..group_pad {
                stb_data.push(0);
            }
        }

        // Create buffer
        let buffer_usage = vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
        let memory_location = MemoryLocation::CpuToGpu;

        let buffer = context.create_buffer(buffer_usage, memory_location, buffer_size as _)?;

        buffer.copy_data_to_buffer(&stb_data)?;

        let address = buffer.get_device_address();

        // see https://nvpro-samples.github.io/vk_raytracing_tutorial_KHR/Images/sbt_0.png
        let raygen_region = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(address)
            .size(raygen_region_size as _)
            .stride(raygen_region_size as _)
            .build();

        let miss_region = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(address + raygen_region.size)
            .size(miss_region_size as _)
            .stride(aligned_handle_size as _)
            .build();

        let hit_region = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(address + raygen_region.size + miss_region.size)
            .size(hit_region_size as _)
            .stride(aligned_handle_size as _)
            .build();

        Ok(Self {
            _buffer: buffer,
            raygen_region,
            miss_region,
            hit_region,
        })
    }
}

impl Context {
    pub fn create_shader_binding_table(
        &self,
        pipeline: &RayTracingPipeline,
    ) -> Result<ShaderBindingTable> {
        let ray_tracing = self.ray_tracing.as_ref().expect(
            "Cannot call Context::create_shader_binding_table when ray tracing is not enabled",
        );

        ShaderBindingTable::new(self, ray_tracing, pipeline)
    }
}
