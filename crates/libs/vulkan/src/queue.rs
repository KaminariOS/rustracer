use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use crate::{device::Device, CommandBuffer, Fence, Semaphore};

#[derive(Debug, Clone, Copy)]
pub struct QueueFamily {
    pub index: u32,
    pub(crate) inner: vk::QueueFamilyProperties,
    supports_present: bool,
}

impl QueueFamily {
    pub(crate) fn new(
        index: u32,
        inner: vk::QueueFamilyProperties,
        supports_present: bool,
    ) -> Self {
        Self {
            index,
            inner,
            supports_present,
        }
    }

    pub fn supports_compute(&self) -> bool {
        self.inner.queue_flags.contains(vk::QueueFlags::COMPUTE)
    }

    pub fn supports_graphics(&self) -> bool {
        self.inner.queue_flags.contains(vk::QueueFlags::GRAPHICS)
    }

    pub fn supports_present(&self) -> bool {
        self.supports_present
    }

    pub fn has_queues(&self) -> bool {
        self.inner.queue_count > 0
    }

    pub fn supports_timestamp_queries(&self) -> bool {
        self.inner.timestamp_valid_bits > 0
    }
}

pub struct Queue {
    device: Arc<Device>,
    pub inner: vk::Queue,
}

impl Queue {
    pub(crate) fn new(device: Arc<Device>, inner: vk::Queue) -> Self {
        Self { device, inner }
    }

    pub fn submit(
        &self,
        command_buffer: &CommandBuffer,
        wait_semaphore: Option<SemaphoreSubmitInfo>,
        signal_semaphore: Option<SemaphoreSubmitInfo>,
        fence: &Fence,
    ) -> Result<()> {
        let wait_semaphore_submit_info = wait_semaphore.map(|s| {
            vk::SemaphoreSubmitInfo::builder()
                .semaphore(s.semaphore.inner)
                .stage_mask(s.stage_mask)
        });

        let signal_semaphore_submit_info = signal_semaphore.map(|s| {
            vk::SemaphoreSubmitInfo::builder()
                .semaphore(s.semaphore.inner)
                .stage_mask(s.stage_mask)
        });

        let cmd_buffer_submit_info =
            vk::CommandBufferSubmitInfo::builder().command_buffer(command_buffer.inner);

        let submit_info = vk::SubmitInfo2::builder()
            .command_buffer_infos(std::slice::from_ref(&cmd_buffer_submit_info));

        let submit_info = match wait_semaphore_submit_info.as_ref() {
            Some(info) => submit_info.wait_semaphore_infos(std::slice::from_ref(info)),
            None => submit_info,
        };

        let submit_info = match signal_semaphore_submit_info.as_ref() {
            Some(info) => submit_info.signal_semaphore_infos(std::slice::from_ref(info)),
            None => submit_info,
        };

        unsafe {
            self.device.inner.queue_submit2(
                self.inner,
                std::slice::from_ref(&submit_info),
                fence.inner,
            )?
        };

        Ok(())
    }
}

pub struct SemaphoreSubmitInfo<'a> {
    pub semaphore: &'a Semaphore,
    pub stage_mask: vk::PipelineStageFlags2,
}
