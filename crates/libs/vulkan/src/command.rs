use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use crate::{
    device::Device, Buffer, ComputePipeline, Context, DescriptorSet, GraphicsPipeline, Image,
    ImageView, PipelineLayout, QueueFamily, RayTracingContext, RayTracingPipeline,
    ShaderBindingTable, TimestampQueryPool,
};

pub struct CommandPool {
    device: Arc<Device>,
    ray_tracing: Option<Arc<RayTracingContext>>,
    pub inner: vk::CommandPool,
}

impl CommandPool {
    pub(crate) fn new(
        device: Arc<Device>,
        ray_tracing: Option<Arc<RayTracingContext>>,
        queue_family: QueueFamily,
        flags: Option<vk::CommandPoolCreateFlags>,
    ) -> Result<Self> {
        let flags = flags.unwrap_or_else(vk::CommandPoolCreateFlags::empty);

        let command_pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family.index)
            .flags(flags);
        let inner = unsafe { device.inner.create_command_pool(&command_pool_info, None)? };

        Ok(Self {
            device,
            ray_tracing,
            inner,
        })
    }

    pub fn allocate_command_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<CommandBuffer>> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.inner)
            .level(level)
            .command_buffer_count(count);

        let buffers = unsafe { self.device.inner.allocate_command_buffers(&allocate_info)? };
        let buffers = buffers
            .into_iter()
            .map(|inner| CommandBuffer {
                device: self.device.clone(),
                ray_tracing: self.ray_tracing.clone(),
                inner,
            })
            .collect();

        Ok(buffers)
    }

    pub fn allocate_command_buffer(&self, level: vk::CommandBufferLevel) -> Result<CommandBuffer> {
        let buffers = self.allocate_command_buffers(level, 1)?;
        let buffer = buffers.into_iter().next().unwrap();

        Ok(buffer)
    }

    pub fn free_command_buffers(&self, buffer: &[CommandBuffer]) {
        let buffs = buffer.iter().map(|b| b.inner).collect::<Vec<_>>();
        unsafe { self.device.inner.free_command_buffers(self.inner, &buffs) };
    }

    pub fn free_command_buffer(&self, buffer: &CommandBuffer) -> Result<()> {
        let buffs = [buffer.inner];
        unsafe { self.device.inner.free_command_buffers(self.inner, &buffs) };

        Ok(())
    }
}

impl Context {
    pub fn create_command_pool(
        &self,
        queue_family: QueueFamily,
        flags: Option<vk::CommandPoolCreateFlags>,
    ) -> Result<CommandPool> {
        CommandPool::new(
            self.device.clone(),
            self.ray_tracing.clone(),
            queue_family,
            flags,
        )
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_command_pool(self.inner, None) };
    }
}

pub struct CommandBuffer {
    device: Arc<Device>,
    ray_tracing: Option<Arc<RayTracingContext>>,
    pub inner: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn begin(&self, flags: Option<vk::CommandBufferUsageFlags>) -> Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(flags.unwrap_or(vk::CommandBufferUsageFlags::empty()));
        unsafe {
            self.device
                .inner
                .begin_command_buffer(self.inner, &begin_info)?
        };

        Ok(())
    }

    pub fn end(&self) -> Result<()> {
        unsafe { self.device.inner.end_command_buffer(self.inner)? };

        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        unsafe {
            self.device
                .inner
                .reset_command_buffer(self.inner, vk::CommandBufferResetFlags::empty())?
        };

        Ok(())
    }

    pub fn bind_rt_pipeline(&self, pipeline: &RayTracingPipeline) {
        unsafe {
            self.device.inner.cmd_bind_pipeline(
                self.inner,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                pipeline.inner,
            )
        }
    }

    pub fn bind_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {
        unsafe {
            self.device.inner.cmd_bind_pipeline(
                self.inner,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.inner,
            )
        }
    }

    pub fn bind_compute_pipeline(&self, pipeline: &ComputePipeline) {
        unsafe {
            self.device.inner.cmd_bind_pipeline(
                self.inner,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.inner,
            )
        }
    }

    pub fn bind_vertex_buffer(&self, vertex_buffer: &Buffer) {
        unsafe {
            self.device
                .inner
                .cmd_bind_vertex_buffers(self.inner, 0, &[vertex_buffer.inner], &[0])
        };
    }

    pub fn draw(&self, vertex_count: u32) {
        unsafe {
            self.device
                .inner
                .cmd_draw(self.inner, vertex_count, 1, 0, 0)
        };
    }

    pub fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.device
                .inner
                .cmd_dispatch(self.inner, group_count_x, group_count_y, group_count_z);
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        bind_point: vk::PipelineBindPoint,
        layout: &PipelineLayout,
        first_set: u32,
        sets: &[&DescriptorSet],
    ) {
        let sets = sets.iter().map(|s| s.inner).collect::<Vec<_>>();
        unsafe {
            self.device.inner.cmd_bind_descriptor_sets(
                self.inner,
                bind_point,
                layout.inner,
                first_set,
                &sets,
                &[],
            )
        }
    }

    pub fn pipeline_buffer_barriers(&self, barriers: &[BufferBarrier]) {
        let barriers = barriers
            .iter()
            .map(|b| {
                vk::BufferMemoryBarrier2::builder()
                    .src_stage_mask(b.src_stage_mask)
                    .src_access_mask(b.src_access_mask)
                    .dst_stage_mask(b.dst_stage_mask)
                    .dst_access_mask(b.dst_access_mask)
                    .buffer(b.buffer.inner)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)
                    .build()
            })
            .collect::<Vec<_>>();

        let dependency_info = vk::DependencyInfo::builder().buffer_memory_barriers(&barriers);

        unsafe {
            self.device
                .inner
                .cmd_pipeline_barrier2(self.inner, &dependency_info)
        };
    }

    pub fn copy_buffer(&self, src_buffer: &Buffer, dst_buffer: &Buffer) {
        unsafe {
            let region = vk::BufferCopy::builder().size(src_buffer.size);
            self.device.inner.cmd_copy_buffer(
                self.inner,
                src_buffer.inner,
                dst_buffer.inner,
                std::slice::from_ref(&region),
            )
        };
    }

    pub fn pipeline_image_barriers(&self, barriers: &[ImageBarrier]) {
        let barriers = barriers
            .iter()
            .map(|b| {
                vk::ImageMemoryBarrier2::builder()
                    .src_stage_mask(b.src_stage_mask)
                    .src_access_mask(b.src_access_mask)
                    .old_layout(b.old_layout)
                    .dst_stage_mask(b.dst_stage_mask)
                    .dst_access_mask(b.dst_access_mask)
                    .new_layout(b.new_layout)
                    .image(b.image.inner)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .build()
            })
            .collect::<Vec<_>>();

        let dependency_info = vk::DependencyInfo::builder().image_memory_barriers(&barriers);

        unsafe {
            self.device
                .inner
                .cmd_pipeline_barrier2(self.inner, &dependency_info)
        };
    }

    pub fn copy_image(
        &self,
        src_image: &Image,
        src_layout: vk::ImageLayout,
        dst_image: &Image,
        dst_layout: vk::ImageLayout,
    ) {
        let region = vk::ImageCopy::builder()
            .src_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                mip_level: 0,
                layer_count: 1,
            })
            .dst_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                mip_level: 0,
                layer_count: 1,
            })
            .extent(vk::Extent3D {
                width: src_image.extent.width,
                height: src_image.extent.height,
                depth: 1,
            });

        unsafe {
            self.device.inner.cmd_copy_image(
                self.inner,
                src_image.inner,
                src_layout,
                dst_image.inner,
                dst_layout,
                std::slice::from_ref(&region),
            )
        };
    }

    pub fn copy_buffer_to_image(&self, src: &Buffer, dst: &Image, layout: vk::ImageLayout) {
        let region = vk::BufferImageCopy::builder()
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_extent(dst.extent);

        unsafe {
            self.device.inner.cmd_copy_buffer_to_image(
                self.inner,
                src.inner,
                dst.inner,
                layout,
                std::slice::from_ref(&region),
            );
        };
    }

    pub fn build_acceleration_structures(
        &self,
        as_build_geo_info: &vk::AccelerationStructureBuildGeometryInfoKHR,
        as_build_range_info: &[vk::AccelerationStructureBuildRangeInfoKHR],
    ) {
        let ray_tracing = self.ray_tracing.as_ref().expect(
            "Cannot call CommandBuffer::build_acceleration_structures when ray tracing is not enabled",
        );

        unsafe {
            ray_tracing
                .acceleration_structure_fn
                .cmd_build_acceleration_structures(
                    self.inner,
                    std::slice::from_ref(as_build_geo_info),
                    std::slice::from_ref(&as_build_range_info),
                )
        };
    }

    pub fn trace_rays(&self, shader_binding_table: &ShaderBindingTable, width: u32, height: u32) {
        let ray_tracing = self
            .ray_tracing
            .as_ref()
            .expect("Cannot call CommandBuffer::trace_rays when ray tracing is not enabled");

        unsafe {
            ray_tracing.pipeline_fn.cmd_trace_rays(
                self.inner,
                &shader_binding_table.raygen_region,
                &shader_binding_table.miss_region,
                &shader_binding_table.hit_region,
                &vk::StridedDeviceAddressRegionKHR::builder(),
                width,
                height,
                1,
            )
        };
    }

    pub fn begin_rendering(
        &self,
        image_view: &ImageView,
        extent: vk::Extent2D,
        load_op: vk::AttachmentLoadOp,
        clear_color: Option<[f32; 4]>,
    ) {
        let color_attachment_info = vk::RenderingAttachmentInfo::builder()
            .image_view(image_view.inner)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(load_op)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color.unwrap_or([1.0; 4]),
                },
            });

        let rendering_info = vk::RenderingInfo::builder()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment_info));

        unsafe {
            self.device
                .inner
                .cmd_begin_rendering(self.inner, &rendering_info)
        };
    }

    pub fn end_rendering(&self) {
        unsafe { self.device.inner.cmd_end_rendering(self.inner) };
    }

    pub fn set_viewport(&self, extent: vk::Extent2D) {
        unsafe {
            self.device.inner.cmd_set_viewport(
                self.inner,
                0,
                &[vk::Viewport {
                    width: extent.width as _,
                    height: extent.height as _,
                    max_depth: 1.0,
                    ..Default::default()
                }],
            )
        };
    }

    pub fn set_scissor(&self, extent: vk::Extent2D) {
        unsafe {
            self.device.inner.cmd_set_scissor(
                self.inner,
                0,
                &[vk::Rect2D {
                    extent,
                    ..Default::default()
                }],
            )
        };
    }

    pub fn reset_all_timestamp_queries_from_pool<const C: usize>(
        &self,
        pool: &TimestampQueryPool<C>,
    ) {
        unsafe {
            self.device
                .inner
                .cmd_reset_query_pool(self.inner, pool.inner, 0, C as _);
        }
    }

    pub fn write_timestamp<const C: usize>(
        &self,
        stage: vk::PipelineStageFlags2,
        pool: &TimestampQueryPool<C>,
        query_index: u32,
    ) {
        assert!(query_index < C as _, "Query index must be < {C}");

        unsafe {
            self.device
                .inner
                .cmd_write_timestamp2(self.inner, stage, pool.inner, query_index)
        }
    }
}

#[derive(Clone, Copy)]
pub struct BufferBarrier<'a> {
    pub buffer: &'a Buffer,
    pub src_access_mask: vk::AccessFlags2,
    pub dst_access_mask: vk::AccessFlags2,
    pub src_stage_mask: vk::PipelineStageFlags2,
    pub dst_stage_mask: vk::PipelineStageFlags2,
}

#[derive(Clone, Copy)]
pub struct ImageBarrier<'a> {
    pub image: &'a Image,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
    pub src_access_mask: vk::AccessFlags2,
    pub dst_access_mask: vk::AccessFlags2,
    pub src_stage_mask: vk::PipelineStageFlags2,
    pub dst_stage_mask: vk::PipelineStageFlags2,
}
