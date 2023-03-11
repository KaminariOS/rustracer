use app::anyhow::Result;
use app::vulkan::ash::vk::{self};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::{vulkan::*, BaseApp};
use app::{App};
use std::mem::{size_of,};
use std::time::Duration;
use app::types::*;
use app::camera::Camera;

mod ubo;
mod gui_state;
mod acceleration_structure;
mod model;
mod pipeline_res;
mod desc_sets;

use acceleration_structure::*;
use model::*;
use gui_state::Gui;
use ubo::UniformBufferObject;
use pipeline_res::*;
use desc_sets::*;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const APP_NAME: &str = "Ray traced cornell box";

const MODEL_PATH: &str = "./assets/models/cornellBox.gltf";
const ACC_BIND: u32 = 8;
const STORAGE_BIND: u32 = 1;
const ENABLE_RAYTRACING: bool = true;

fn main() -> Result<()> {
    app::run::<CornellBox>(APP_NAME, WIDTH, HEIGHT, ENABLE_RAYTRACING)
}

struct CornellBox {
    ubo_buffer: Buffer,
    _model: Model,
    _bottom_as: BottomAS,
    _top_as: TopAS,
    pipeline_res: PipelineRes,
    sbt: ShaderBindingTable,
    descriptor_res: DescriptorRes,
    total_number_of_samples: u32,
    gui_state: Option<Gui>,
    old_camera: Option<Camera>,
}

impl App for CornellBox {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<UniformBufferObject>() as _,
        )?;

        let model = create_model(context)?;

        let bottom_as = create_bottom_as(context, &model)?;

        let top_as = create_top_as(context, &bottom_as)?;

        let pipeline_res = create_pipeline(context, &model)?;

        let sbt = context.create_shader_binding_table(&pipeline_res.pipeline)?;

        let descriptor_res = create_descriptor_sets(
            context,
            &pipeline_res,
            &model,
            &bottom_as,
            &top_as,
            base.storage_images.as_slice(),
            base.acc_images.as_slice(),
            &ubo_buffer,
        )?;

        base.camera.position = Point::new(-2.0, 1.5, 2.0);
        base.camera.direction = Vec3::new(2.0, -0.5, -2.0);

        Ok(Self {
            ubo_buffer,
            _model: model,
            _bottom_as: bottom_as,
            _top_as: top_as,
            pipeline_res,
            sbt,
            descriptor_res,
            total_number_of_samples: 0,
            old_camera: None,
            gui_state: None
        })
    }

    fn update(
        &mut self,
        base: &BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _image_index: usize,
        _: Duration,
    ) -> Result<()> {
        self.state_change(base, gui);
        let view = base.camera.view_matrix();
        let inverted_view = view.try_inverse().expect("Should be invertible");

        let proj = base.camera.projection_matrix();
        let inverted_proj = proj.try_inverse().expect("Should be invertible");
        let number_of_samples = if gui.max_number_of_samples <= self.total_number_of_samples {0} else {
            (gui.max_number_of_samples - self.total_number_of_samples).min(gui.number_of_samples)
        };
        // println!("nums {} total: {}", number_of_samples, self.total_number_of_samples);
        if !gui.acc || gui.heatmap {
            self.total_number_of_samples = 0;
        }
        self.total_number_of_samples += number_of_samples;

        let ubo = UniformBufferObject {
            model_view: view,
            projection: proj,
            model_view_inverse: inverted_view,
            projection_inverse: inverted_proj,
            aperture: gui.aperture,
            focus_distance: gui.focus_distance,
            heatmap_scale: gui.heatmap_scale,
            total_number_of_samples: self.total_number_of_samples,
            number_of_samples,
            number_of_bounces: gui.number_of_bounces,
            random_seed: 3,
            has_sky: gui.sky.into(),
            show_heatmap: gui.heatmap.into(),
        };

        self.ubo_buffer.copy_data_to_buffer(&[ubo])?;

        Ok(())
    }

    fn record_raytracing_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let static_set = &self.descriptor_res.static_set;
        let dynamic_set = &self.descriptor_res.dynamic_sets[image_index];

        buffer.bind_rt_pipeline(&self.pipeline_res.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            &self.pipeline_res.pipeline_layout,
            0,
            &[static_set, dynamic_set],
        );

        buffer.trace_rays(
            &self.sbt,
            base.swapchain.extent.width,
            base.swapchain.extent.height,
        );

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        base.storage_images
            .iter()
            .enumerate()
            .for_each(|(index, img)| {
                let set = &self.descriptor_res.dynamic_sets[index];

                set.update(&[WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &img.view,
                    },
                }]);
            });

        for i in 0..base.storage_images.len() {
            base.acc_images
                .iter()
                .enumerate()
                .for_each(|(_index, img)| {
                    let set = &self.descriptor_res.dynamic_sets[i];

                    set.update(&[WriteDescriptorSet {
                        binding: ACC_BIND,
                        kind: WriteDescriptorSetKind::StorageImage {
                            layout: vk::ImageLayout::GENERAL,
                            view: &img.view,
                        },
                    }]);
                });
        }
        Ok(())
    }

    fn state_change(&mut self, base: &BaseApp<Self>, gui_state: &<Self as App>::Gui) {
        if self.old_camera.is_none() {
            self.old_camera = Some(base.camera);
        }
        if self.gui_state.is_none() {
            self.gui_state = Some(*gui_state);
        }

        if self.old_camera.filter(|x| *x != base.camera).is_some() {
            self.old_camera = Some(base.camera);
            self.total_number_of_samples = 0;
        }

        if self.gui_state.filter(|x| x != gui_state).is_some() {
            self.gui_state = Some(*gui_state);
            self.total_number_of_samples = 0;
        }
    }
}



#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GeometryInfo {
    transform: Mat4,
    base_color: [f32; 4],
    emissive_factor: [f32; 4],
    base_color_texture_index: i32,
    metallic_factor: f32,
    roughness: f32,
    ior: f32,
    _padding: f32,
    _padding2: f32,
    vertex_offset: u32,
    index_offset: u32,
}


