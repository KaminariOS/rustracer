mod acceleration_structure;
mod pipeline;
mod shader_binding_table;

pub use acceleration_structure::*;
pub use pipeline::*;
pub use shader_binding_table::*;

use ash::{
    extensions::khr::{
        AccelerationStructure as AshAccelerationStructure,
        RayTracingPipeline as AshRayTracingPipeline,
    },
    vk,
};

use crate::{device::Device, instance::Instance, physical_device::PhysicalDevice};

pub struct RayTracingContext {
    pub pipeline_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub pipeline_fn: AshRayTracingPipeline,
    pub acceleration_structure_properties: vk::PhysicalDeviceAccelerationStructurePropertiesKHR,
    pub acceleration_structure_fn: AshAccelerationStructure,
}

impl RayTracingContext {
    pub(crate) fn new(instance: &Instance, pdevice: &PhysicalDevice, device: &Device) -> Self {
        let pipeline_properties =
            unsafe { AshRayTracingPipeline::get_properties(&instance.inner, pdevice.inner) };
        let pipeline_fn = AshRayTracingPipeline::new(&instance.inner, &device.inner);

        let acceleration_structure_properties =
            unsafe { AshAccelerationStructure::get_properties(&instance.inner, pdevice.inner) };
        let acceleration_structure_fn =
            AshAccelerationStructure::new(&instance.inner, &device.inner);

        Self {
            pipeline_properties,
            pipeline_fn,
            acceleration_structure_properties,
            acceleration_structure_fn,
        }
    }
}
