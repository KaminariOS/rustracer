use std::time::Duration;

use app::anyhow::Result;
use app::vulkan::ash::vk;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, CommandBuffer, Context, GraphicsPipeline, GraphicsPipelineCreateInfo,
    GraphicsShaderCreateInfo, PipelineLayout,
};
use app::{App, BaseApp};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Triangle";

fn main() -> Result<()> {
    app::run::<Triangle>(APP_NAME, WIDTH, HEIGHT, false)
}
struct Triangle {
    vertex_buffer: Buffer,
    _pipeline_layout: PipelineLayout,
    pipeline: GraphicsPipeline,
}

impl App for Triangle {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let vertex_buffer = create_vertex_buffer(context)?;

        let pipeline_layout = context.create_pipeline_layout(&[])?;

        let pipeline = create_pipeline(context, &pipeline_layout, base.swapchain.format)?;

        Ok(Self {
            vertex_buffer,
            _pipeline_layout: pipeline_layout,
            pipeline,
        })
    }

    fn update(
        &mut self,
        _: &BaseApp<Self>,
         _: &mut <Self as App>::Gui,
        _: usize,
        _: Duration,
    ) -> Result<()> {
        Ok(())
    }

    fn record_raster_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        buffer.begin_rendering(
            &base.swapchain.views[image_index],
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);
        buffer.draw(3);
        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, _: &BaseApp<Self>) -> Result<()> {
        Ok(())
    }

    fn state_change(&mut self, _base: &BaseApp<Self>, _gui_state: &Self::Gui) {

    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl app::vulkan::Vertex for Vertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: 20,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 8,
            },
        ]
    }
}

fn create_vertex_buffer(context: &Context) -> Result<Buffer> {
    let vertices: [Vertex; 3] = [
        Vertex {
            position: [-1.0, 1.0],
            color: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0],
            color: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.0, -1.0],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let vertex_buffer =
        create_gpu_only_buffer_from_data(context, vk::BufferUsageFlags::VERTEX_BUFFER, &vertices)?;

    Ok(vertex_buffer)
}

fn create_pipeline(
    context: &Context,
    layout: &PipelineLayout,
    color_attachment_format: vk::Format,
) -> Result<GraphicsPipeline> {
    context.create_graphics_pipeline::<Vertex>(
        layout,
        GraphicsPipelineCreateInfo {
            shaders: &[
                GraphicsShaderCreateInfo {
                    source: &include_bytes!("../shaders/shader.vert.spv")[..],
                    stage: vk::ShaderStageFlags::VERTEX,
                },
                GraphicsShaderCreateInfo {
                    source: &include_bytes!("../shaders/shader.frag.spv")[..],
                    stage: vk::ShaderStageFlags::FRAGMENT,
                },
            ],
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            extent: None,
            color_attachment_format,
            color_attachment_blend: None,
            dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
        },
    )
}
