use std::ffi::CStr;

use anyhow::Result;
use ash::{vk, Instance};

use crate::{device::DeviceFeatures, queue::QueueFamily, surface::Surface};

#[derive(Debug, Clone)]
pub struct PhysicalDevice {
    pub(crate) inner: vk::PhysicalDevice,
    pub(crate) name: String,
    pub(crate) device_type: vk::PhysicalDeviceType,
    pub(crate) limits: vk::PhysicalDeviceLimits,
    pub(crate) queue_families: Vec<QueueFamily>,
    pub(crate) supported_extensions: Vec<String>,
    pub(crate) supported_surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub(crate) supported_present_modes: Vec<vk::PresentModeKHR>,
    pub(crate) supported_device_features: DeviceFeatures,
}

impl PhysicalDevice {
    pub(crate) fn new(
        instance: &Instance,
        surface: &Surface,
        inner: vk::PhysicalDevice,
    ) -> Result<Self> {
        let props = unsafe { instance.get_physical_device_properties(inner) };

        let name = unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
                .to_str()
                .unwrap()
                .to_owned()
        };

        let device_type = props.device_type;
        let limits = props.limits;

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(inner) };
        let queue_families = queue_family_properties
            .into_iter()
            .enumerate()
            .map(|(index, p)| {
                let present_support = unsafe {
                    surface.inner.get_physical_device_surface_support(
                        inner,
                        index as _,
                        surface.surface_khr,
                    )?
                };

                Ok(QueueFamily::new(index as _, p, present_support))
            })
            .collect::<Result<_>>()?;

        let extension_properties =
            unsafe { instance.enumerate_device_extension_properties(inner)? };
        let supported_extensions = extension_properties
            .into_iter()
            .map(|p| {
                let name = unsafe { CStr::from_ptr(p.extension_name.as_ptr()) };
                name.to_str().unwrap().to_owned()
            })
            .collect();

        let supported_surface_formats = unsafe {
            surface
                .inner
                .get_physical_device_surface_formats(inner, surface.surface_khr)?
        };

        let supported_present_modes = unsafe {
            surface
                .inner
                .get_physical_device_surface_present_modes(inner, surface.surface_khr)?
        };

        let mut ray_tracing_feature = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
        let mut acceleration_struct_feature =
            vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();
        let mut features12 = vk::PhysicalDeviceVulkan12Features::builder()
            .runtime_descriptor_array(true)
            .buffer_device_address(true);
        let mut features13 = vk::PhysicalDeviceVulkan13Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut ray_tracing_feature)
            .push_next(&mut acceleration_struct_feature)
            .push_next(&mut features12)
            .push_next(&mut features13);
        unsafe { instance.get_physical_device_features2(inner, &mut features) };

        let supported_device_features = DeviceFeatures {
            ray_tracing_pipeline: ray_tracing_feature.ray_tracing_pipeline == vk::TRUE,
            acceleration_structure: acceleration_struct_feature.acceleration_structure == vk::TRUE,
            runtime_descriptor_array: features12.runtime_descriptor_array == vk::TRUE,
            buffer_device_address: features12.buffer_device_address == vk::TRUE,
            dynamic_rendering: features13.dynamic_rendering == vk::TRUE,
            synchronization2: features13.synchronization2 == vk::TRUE,
        };

        Ok(Self {
            inner,
            name,
            device_type,
            limits,
            queue_families,
            supported_extensions,
            supported_surface_formats,
            supported_present_modes,
            supported_device_features,
        })
    }

    pub fn supports_extensions(&self, extensions: &[&str]) -> bool {
        let supported_extensions = self
            .supported_extensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        extensions.iter().all(|e| supported_extensions.contains(e))
    }
}
