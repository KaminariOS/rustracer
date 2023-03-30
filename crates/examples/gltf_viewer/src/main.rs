use app::anyhow::Result;
use app::camera::Camera;
use app::types::*;
use app::vulkan::ash::vk::{self};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::App;
use app::{vulkan::*, BaseApp};
use std::mem::size_of;
use std::time::Duration;

mod desc_sets;
mod gui_state;
mod pipeline_res;
mod ubo;

use crate::gui_state::Scene;
use asset_loader::acceleration_structures::{create_as, TopAS};
use asset_loader::globals::{create_global, Buffers, VkGlobal};
use asset_loader::Doc;
use desc_sets::*;
use gui_state::Gui;
use pipeline_res::*;
use ubo::UniformBufferObject;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const APP_NAME: &str = "Ray traced cornell box";

const AS_BIND: u32 = 0;
const STORAGE_BIND: u32 = 1;
const UNIFORM_BIND: u32 = 2;
const VERTEX_BIND: u32 = 3;
const INDEX_BIND: u32 = 4;
const GEO_BIND: u32 = 5;
const TEXTURE_BIND: u32 = 6;
const ACC_BIND: u32 = 8;
const MAT_BIND: u32 = 9;
const ENABLE_RAYTRACING: bool = true;

fn main() -> Result<()> {
    app::run::<GltfViewer>(APP_NAME, WIDTH, HEIGHT, ENABLE_RAYTRACING)
}

struct GltfViewer {
    ubo_buffer: Buffer,
    _model: Doc,
    _bottom_as: Vec<AccelerationStructure>,
    _top_as: TopAS,
    pipeline_res: PipelineRes,
    sbt: ShaderBindingTable,
    descriptor_res: DescriptorRes,
    total_number_of_samples: u32,
    gui_state: Option<Gui>,
    old_camera: Option<Camera>,

    buffers: Buffers,
    globals: VkGlobal
}
impl GltfViewer {
    fn new_with_scene(base: &mut BaseApp<Self>, scene: Scene) -> Result<Self> {
        let context = &mut base.context;

        let ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<UniformBufferObject>() as _,
        )?;

        // let model = create_model(context, scene)?;

        let doc = asset_loader::load_file(scene.path())?;

        let globals = create_global(context, &doc)?;
        let buffers = Buffers::new(context, &doc.geo_builder, &globals)?;
        let (blas, tlas) = create_as(context, &doc, &buffers)?;
        // let bottom_as = create_bottom_as(context, &model)?;

        // let top_as = create_top_as(context, &bottom_as)?;

        let pipeline_res = create_pipeline(context, &globals)?;

        let sbt = context.create_shader_binding_table(&pipeline_res.pipeline)?;

        let descriptor_res = create_descriptor_sets(
            context,
            &pipeline_res,
            &globals,
            &tlas,
            base.storage_images.as_slice(),
            base.acc_images.as_slice(),
            &ubo_buffer,
            &buffers,
        )?;

        base.camera.position = Point::new(0., 0.0, 16.0);
        base.camera.direction = Vec3::new(0.0, -0.0, -2.0);

        Ok(Self {
            ubo_buffer,
            _model: doc,
            _bottom_as: blas,
            _top_as: tlas,
            pipeline_res,
            sbt,
            descriptor_res,
            total_number_of_samples: 0,
            old_camera: None,
            gui_state: None,
            buffers,
            globals
        })
    }
}
impl App for GltfViewer {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        Self::new_with_scene(base, Scene::Type59)
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _image_index: usize,
        _: Duration,
    ) -> Result<()> {
        self.state_change(base, gui);
        let view = base.camera.view_matrix() * gui.scale;
        let inverted_view = view.try_inverse().expect("Should be invertible");

        let proj = base.camera.projection_matrix();
        let inverted_proj = proj.try_inverse().expect("Should be invertible");
        let number_of_samples = if gui.max_number_of_samples <= self.total_number_of_samples {
            0
        } else {
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
                    binding: STORAGE_BIND,
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

    fn state_change(&mut self, base: &mut BaseApp<Self>, gui_state: &mut <Self as App>::Gui) {
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

        if let Some(old_state) = self.gui_state.filter(|x| x != gui_state) {
            if old_state.scene != gui_state.scene {
                *self = Self::new_with_scene(base, gui_state.scene).unwrap();
            }
            self.gui_state = Some(*gui_state);
            self.total_number_of_samples = 0;
        }
    }
}