use std::mem::size_of_val;

use anyhow::Result;
use ash::vk;
pub mod platforms;

use gpu_allocator::MemoryLocation;

use crate::{Buffer, BufferBarrier, Context};

pub fn compute_aligned_size(size: u32, alignment: u32) -> u32 {
    (size + (alignment - 1)) & !(alignment - 1)
}

pub fn read_shader_from_bytes(bytes: &[u8]) -> Result<Vec<u32>> {
    let mut cursor = std::io::Cursor::new(bytes);
    Ok(ash::util::read_spv(&mut cursor)?)
}

pub fn create_gpu_only_buffer_from_data<T: Copy>(
    context: &Context,
    usage: vk::BufferUsageFlags,
    data: &[T],
) -> Result<Buffer> {
    let size = size_of_val(data) as _;
    let staging_buffer = context.create_buffer(
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
        size,
    )?;
    staging_buffer.copy_data_to_buffer(data)?;

    let buffer = context.create_buffer(
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        MemoryLocation::GpuOnly,
        size,
    )?;

    context.execute_one_time_commands(|cmd_buffer| {
        cmd_buffer.copy_buffer(&staging_buffer, &buffer);
    })?;

    Ok(buffer)
}


pub fn create_gpu_only_as_buffer_from_data<T: Copy>(
    context: &Context,
    usage: vk::BufferUsageFlags,
    data: &[T],
) -> Result<Buffer> {
    let size = size_of_val(data) as _;
    let staging_buffer = context.create_buffer(
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
        size,
    )?;
    staging_buffer.copy_data_to_buffer(data)?;

    let buffer = context.create_buffer(
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        MemoryLocation::GpuOnly,
        size,
    )?;

    context.execute_one_time_commands(|cmd_buffer| {
        cmd_buffer.copy_buffer(&staging_buffer, &buffer);
        cmd_buffer.pipeline_buffer_barriers(&[
                    BufferBarrier {
                        buffer: &buffer,
                        src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                        dst_access_mask: vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR | vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR,
                        src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                        dst_stage_mask: vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR
                    },
        ])
    })?;

    Ok(buffer)
}
