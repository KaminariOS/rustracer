use std::{ffi::CString, sync::Arc};

use anyhow::Result;
use ash::vk;

use crate::{device::Device, Context, PipelineLayout, ShaderModule};

pub struct GraphicsPipeline {
    device: Arc<Device>,
    pub(crate) inner: vk::Pipeline,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsPipelineCreateInfo<'a> {
    pub shaders: &'a [GraphicsShaderCreateInfo<'a>],
    pub primitive_topology: vk::PrimitiveTopology,
    pub extent: Option<vk::Extent2D>,
    pub color_attachment_format: vk::Format,
    pub color_attachment_blend: Option<vk::PipelineColorBlendAttachmentState>,
    pub dynamic_states: Option<&'a [vk::DynamicState]>,
}

pub trait Vertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription>;
    fn attributes() -> Vec<vk::VertexInputAttributeDescription>;
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsShaderCreateInfo<'a> {
    pub source: &'a [u8],
    pub stage: vk::ShaderStageFlags,
}

impl GraphicsPipeline {
    pub(crate) fn new<V: Vertex>(
        device: Arc<Device>,
        layout: &PipelineLayout,
        create_info: GraphicsPipelineCreateInfo,
    ) -> Result<Self> {
        // shaders
        let mut shader_modules = vec![];
        let mut shader_stages_infos = vec![];

        let entry_point_name = CString::new("main").unwrap();

        for shader in create_info.shaders.iter() {
            let module = ShaderModule::from_bytes(device.clone(), shader.source)?;

            let stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(shader.stage)
                .module(module.inner)
                .name(&entry_point_name)
                .build();

            shader_modules.push(module);
            shader_stages_infos.push(stage);
        }

        // vertex
        let vertex_bindings = V::bindings();
        let vertex_attributes = V::attributes();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_bindings)
            .vertex_attribute_descriptions(&vertex_attributes);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(create_info.primitive_topology)
            .primitive_restart_enable(false);

        // viewport/scissors
        let viewports = create_info
            .extent
            .map(|e| {
                vec![vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: e.width as _,
                    height: e.height as _,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }]
            })
            .unwrap_or_default();
        let scissors = create_info
            .extent
            .map(|e| {
                vec![vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: e,
                }]
            })
            .unwrap_or_default();

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .viewport_count(1)
            .scissors(&scissors)
            .scissor_count(1);

        // raster
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        // msaa
        let multisampling_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        // blending
        let color_blend_attachment =
            create_info
                .color_attachment_blend
                .unwrap_or(vk::PipelineColorBlendAttachmentState {
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                    ..Default::default()
                });
        let color_blend_attachments = [color_blend_attachment];
        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        // dynamic states
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(create_info.dynamic_states.unwrap_or(&[]));

        // dynamic rendering
        let color_attachment_formats = [create_info.color_attachment_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&color_attachment_formats);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages_infos)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .dynamic_state(&dynamic_state_info)
            .layout(layout.inner)
            .push_next(&mut rendering_info);

        let inner = unsafe {
            device
                .inner
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|e| e.1)?[0]
        };

        Ok(Self { device, inner })
    }
}

impl Context {
    pub fn create_graphics_pipeline<V: Vertex>(
        &self,
        layout: &PipelineLayout,
        create_info: GraphicsPipelineCreateInfo,
    ) -> Result<GraphicsPipeline> {
        GraphicsPipeline::new::<V>(self.device.clone(), layout, create_info)
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_pipeline(self.inner, None) };
    }
}
