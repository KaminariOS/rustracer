use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use crate::{Context, Device};

pub struct TimestampQueryPool<const C: usize> {
    device: Arc<Device>,
    pub(crate) inner: vk::QueryPool,
    timestamp_period: f64,
}

impl<const C: usize> TimestampQueryPool<C> {
    pub(crate) fn new(device: Arc<Device>, timestamp_period: f64) -> Result<Self> {
        let create_info = vk::QueryPoolCreateInfo::builder()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(C as _);

        let inner = unsafe { device.inner.create_query_pool(&create_info, None)? };

        Ok(Self {
            device,
            inner,
            timestamp_period,
        })
    }
}

impl Context {
    pub fn create_timestamp_query_pool<const C: usize>(&self) -> Result<TimestampQueryPool<C>> {
        TimestampQueryPool::new(
            self.device.clone(),
            self.physical_device.limits.timestamp_period as _,
        )
    }
}

impl<const C: usize> Drop for TimestampQueryPool<C> {
    fn drop(&mut self) {
        unsafe {
            self.device.inner.destroy_query_pool(self.inner, None);
        }
    }
}

impl<const C: usize> TimestampQueryPool<C> {
    pub fn reset_all(&self) {
        unsafe {
            self.device.inner.reset_query_pool(self.inner, 0, C as _);
        }
    }

    pub fn wait_for_all_results(&self) -> Result<[u64; C]> {
        let mut data = [0u64; C];

        unsafe {
            self.device.inner.get_query_pool_results(
                self.inner,
                0,
                C as _,
                &mut data,
                vk::QueryResultFlags::WAIT | vk::QueryResultFlags::TYPE_64,
            )?;
        }

        let mut result = [0u64; C];
        for (index, timestamp) in data.iter().enumerate() {
            result[index] = (*timestamp as f64 * self.timestamp_period) as u64;
        }

        Ok(result)
    }
}
