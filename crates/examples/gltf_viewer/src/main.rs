use std::default::Default;
use app::anyhow::Result;
use app::camera::Camera;
use app::types::*;
use app::vulkan::ash::vk::{self};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::App;
use app::{vulkan::*, BaseApp};
use std::mem::size_of;
use std::time::{Duration, Instant};

mod desc_sets;
mod gui_state;
mod pipeline_res;
mod ubo;

use crate::gui_state::{Scene, Skybox};
use asset_loader::acceleration_structures::{BlasInput, create_as, create_top_as, TopAS};
use asset_loader::globals::{create_global, Buffers, VkGlobal, SkyboxResource};
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
const DLIGHT_BIND: u32 = 10;
const PLIGHT_BIND: u32 = 11;
const SKYBOX_BIND: u32 = 12;
const ENABLE_RAYTRACING: bool = true;

fn main() -> Result<()> {
    app::run::<GltfViewer>(APP_NAME, WIDTH, HEIGHT, ENABLE_RAYTRACING)
}

struct GltfViewer {
    ubo_buffer: Buffer,
    doc: Doc,
    _bottom_as: Vec<AccelerationStructure>,
    blas_inputs: Vec<BlasInput>,
    _top_as: TopAS,
    pipeline_res: PipelineRes,
    sbt: ShaderBindingTable,
    descriptor_res: DescriptorRes,
    total_number_of_samples: u32,
    prev_gui_state: Option<Gui>,
    old_camera: Option<Camera>,

    buffers: Buffers,
    globals: VkGlobal,
    clock: Instant,
    last_update: Instant,
}
impl GltfViewer {
    fn new_with_scene(base: &mut BaseApp<Self>, scene: Scene, skybox: Skybox) -> Result<Self> {
        let context = &mut base.context;

        let ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<UniformBufferObject>() as _,
        )?;
        let doc = asset_loader::load_file(scene.path())?;
        let skybox = SkyboxResource::new(context, skybox.path())?;
        let globals = create_global(context, &doc, skybox)?;
        let buffers = Buffers::new(context, &doc.geo_builder, &globals)?;
        let (blas, blas_inputs, tlas) = create_as(context, &doc, &buffers, if doc.static_scene() {
            vk::BuildAccelerationStructureFlagsKHR::empty()
        } else {
            vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE
        })?;
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
            doc: doc,
            _bottom_as: blas,
            blas_inputs,
            _top_as: tlas,
            pipeline_res,
            sbt,
            descriptor_res,
            total_number_of_samples: 0,
            old_camera: None,
            prev_gui_state: None,
            buffers,
            globals,
            clock: Instant::now(),
            last_update: Instant::now(),
        })
    }
}
impl App for GltfViewer {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        Self::new_with_scene(base, Default::default(), Default::default())
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
        let number_of_samples = gui.get_number_of_samples(self.total_number_of_samples);
        // println!("nums {} total: {}", number_of_samples, self.total_number_of_samples);
        if !gui.acc() {
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
            heatmap_scale: gui.map_scale,
            total_number_of_samples: self.total_number_of_samples,
            number_of_samples,
            number_of_bounces: gui.get_bounce(),
            random_seed: 3,
            has_sky: gui.sky.into(),
            mapping: gui.mapping as _,
            antialiasing: gui.antialiasing.into(),
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
        if self.prev_gui_state.is_none() {
            self.prev_gui_state = Some(*gui_state);
        }

        if self.old_camera.filter(|x| *x != base.camera).is_some() {
            self.old_camera = Some(base.camera);
            self.total_number_of_samples = 0;
        }

        if let Some(old_state) = self.prev_gui_state.filter(|x| x != gui_state) {
            if old_state.scene != gui_state.scene {
                base.wait_for_gpu().unwrap();
                *self = Self::new_with_scene(base, gui_state.scene, gui_state.skybox).unwrap();
            }
            if old_state.skybox != gui_state.skybox {
                let skybox = SkyboxResource::new(&base.context, gui_state.skybox.path()).unwrap();
                skybox.update_desc(&self.descriptor_res.static_set, SKYBOX_BIND);
                self.globals.skybox = skybox;
            }
            self.prev_gui_state = Some(*gui_state);
            self.total_number_of_samples = 0;
        }

        if !self.doc.static_scene() && gui_state.animation && self.need_update() {
            self.last_update = Instant::now();
            let t = self.clock.elapsed().as_secs_f32() * gui_state.animation_speed;
            self.doc.animate(t);
            let tlas = create_top_as(&base.context, &self.doc, &self._bottom_as, vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE).unwrap();
            self.update_tlas(tlas);
        }
    }


}

impl GltfViewer {
    fn update_tlas(&mut self, tlas: TopAS) {
        self.descriptor_res.static_set.update(&[
            WriteDescriptorSet {
                binding: AS_BIND,
                kind: WriteDescriptorSetKind::AccelerationStructure {
                    acceleration_structure: &tlas.inner,
                },
            },
        ]);
        self._top_as = tlas;
    }

    fn need_update(&self) -> bool {
        self.last_update.elapsed().as_secs_f32() >= 1./60.
    }
}