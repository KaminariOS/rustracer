use app::load_spv;
use app::vulkan::{Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo, Context, DescriptorSetLayout, PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind};
use app::vulkan::ash::vk;
use crate::desc_sets::DescriptorRes;
use crate::{ANIMATION_BIND, SKIN_BIND, VERTEX_BIND};
use app::anyhow::Result;
use asset_loader::globals::Buffers;

const DISPATCH_SIZE: u32 = 256;
pub struct ComputeUnit {
    pipeline: ComputePipelineRes,
    descriptor_res: DescriptorRes,
    // skins_buffer: Buffer,
    // animation_buffer: Buffer,
}

impl ComputeUnit {
    pub fn new(context: &Context, buffers: &Buffers) -> Result<Self> {
        let pipeline = create_compute_pipeline(context)?;
        let descriptor_res = create_descriptor_sets(context, &pipeline, buffers)?;
        Ok(Self {
            pipeline,
            descriptor_res,
        })
    }

    pub fn dispatch(&self,
                    context: &Context,
                    buffers: &Buffers,
                    vertex_count: u32
    ) -> Result<()> {
        let cmd_buffer = context
            .command_pool
            .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)?;
        cmd_buffer.begin(Some(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT))?;
        let static_set = &self.descriptor_res.static_set;
        cmd_buffer.bind_compute_pipeline(&self.pipeline.pipeline);
        cmd_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline.pipeline_layout,
            0,
            &[static_set],
        );
        cmd_buffer.dispatch((vertex_count / DISPATCH_SIZE) + 1, 1, 1,);
        unsafe {
            context.device.inner.cmd_pipeline_barrier2(
                cmd_buffer.inner,
                &vk::DependencyInfo::builder()
                    .memory_barriers(&[vk::MemoryBarrier2::builder()
                        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                        .dst_access_mask(vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR)
                        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
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
        // // Free
        context.command_pool.free_command_buffer(&cmd_buffer)?;
        Ok(())
    }
}


pub fn create_compute_pipeline(context: &Context) -> Result<ComputePipelineRes> {
    let shader = load_spv("AnimationCompute.comp.spv");
    let info = ComputePipelineCreateInfo {
        shader_source: &shader,
    };
    let stage_flag = vk::ShaderStageFlags::COMPUTE;
    let layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(VERTEX_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(stage_flag)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(ANIMATION_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(stage_flag)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(SKIN_BIND)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(stage_flag)
            .build(),
    ];
    let dsl = context.create_descriptor_set_layout(&layout_bindings)?;
    let pipeline_layout = context.create_pipeline_layout(&[&dsl])?;
    let pipeline = ComputePipeline::new(context.device.clone(), &pipeline_layout, info)?;
    Ok(ComputePipelineRes {
        pipeline,
        pipeline_layout,
        dsl,
    })
}

pub struct ComputePipelineRes {
    pub(crate) pipeline: ComputePipeline,
    pub(crate) pipeline_layout: PipelineLayout,
    pub(crate) dsl: DescriptorSetLayout,
}

pub fn create_descriptor_sets(
    context: &Context,
    pipeline_res: &ComputePipelineRes,
    buffers: &Buffers,
) -> Result<DescriptorRes> {

    let pool_sizes = [
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(3)
            .build(),
    ];

    let pool = context.create_descriptor_pool(1, &pool_sizes)?;

    let static_set = pool.allocate_set(&pipeline_res.dsl)?;
    let (skins, ani) = buffers.animation_buffers.as_ref().unwrap();
    static_set.update(&[
        WriteDescriptorSet {
            binding: VERTEX_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &buffers.vertex_buffer,
            },
        },
        WriteDescriptorSet {
            binding: SKIN_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: skins,
            },
        },
        WriteDescriptorSet {
            binding: ANIMATION_BIND,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: ani,
            },
        },
    ]);
    Ok(DescriptorRes {
        _pool: pool,
        dynamic_sets: vec![],
        static_set,
    })
}

