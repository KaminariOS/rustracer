use app::anyhow::Result;
use app::camera::Camera;
use std::default::Default;

use app::vulkan::ash::vk::{self};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::{vulkan::*, BaseApp};
use app::{App, FrameStats};
use std::mem::size_of;
use std::path::PathBuf;

use log::info;
use std::time::Instant;

mod args;
mod compute_unit;
mod desc_sets;
mod gui_state;
mod loader;
mod pipeline_res;
mod ubo;

use crate::args::Args;
use crate::compute_unit::ComputeUnit;
use crate::gui_state::{Scene, Skybox};
use crate::loader::Loader;
use asset_loader::acceleration_structures::{create_as, create_top_as, TopAS};
use asset_loader::globals::{create_global, Buffers, SkyboxResource, VkGlobal};
use asset_loader::light::LightRaw;
use asset_loader::{load_file, Doc};
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
const ANIMATION_BIND: u32 = 13;
const SKIN_BIND: u32 = 14;
const ENABLE_RAYTRACING: bool = true;

fn main() -> Result<()> {
    app::run::<GltfViewer>(APP_NAME, WIDTH, HEIGHT, ENABLE_RAYTRACING)
}

struct GltfViewerInner {
    doc: Doc,
    _bottom_as: Vec<AccelerationStructure>,
    top_as: TopAS,
    pipeline_res: PipelineRes,
    sbt: ShaderBindingTable,
    descriptor_res: DescriptorRes,
    fully_opaque: bool,
    buffers: Buffers,
    globals: VkGlobal,
    compute_unit: Option<ComputeUnit>,
}

impl GltfViewerInner {
    fn new(
        base: &BaseApp<GltfViewer>,
        doc: Doc,
        ubo_buffer: &Buffer,
        skybox: &SkyboxResource,
    ) -> Result<Self> {
        let context = &base.context;
        let globals = create_global(context, &doc)?;
        let fully_opaque = doc.geo_builder.fully_opaque();

        let buffers = Buffers::new(context, &doc.geo_builder, &globals)?;
        let compute_unit = if buffers.animation_buffers.is_some() {
            let compute_unit = ComputeUnit::new(context, &buffers)?;
            compute_unit.dispatch(context, &buffers, doc.geo_builder.vertices.len() as u32)?;
            Some(compute_unit)
        } else {
            None
        };

        let (_bottom_as, top_as) = create_as(
            context,
            &doc,
            &buffers,
            vk::BuildAccelerationStructureFlagsKHR::empty(),
        )?;
        let pipeline_res = create_pipeline(context, &globals, fully_opaque)?;

        let sbt = context.create_shader_binding_table(&pipeline_res.pipeline)?;

        let descriptor_res = create_descriptor_sets(
            &base.context,
            &pipeline_res,
            &globals,
            &top_as,
            base.storage_images.as_slice(),
            base.acc_images.as_slice(),
            ubo_buffer,
            &buffers,
        )?;

        skybox.update_desc(&descriptor_res.static_set, SKYBOX_BIND);

        Ok(GltfViewerInner {
            doc,
            _bottom_as,
            top_as,
            pipeline_res,
            sbt,
            descriptor_res,
            fully_opaque,
            buffers,
            globals,
            compute_unit,
        })
    }
}

struct GltfViewer {
    ubo_buffer: Buffer,
    total_number_of_samples: u32,
    prev_gui_state: Option<Gui>,
    old_camera: Option<Camera>,

    clock: Instant,
    last_update: Instant,
    loader: Loader,
    inner: Vec<GltfViewerInner>,
    skybox: SkyboxResource,
}

impl GltfViewer {
    fn new_with_scene(base: &BaseApp<Self>, scene: Scene, loader: Loader) -> Result<Self> {
        let doc = load_file(scene.path())?;
        Self::new_with_doc(base, doc, loader)
    }

    fn new_with_doc(base: &BaseApp<Self>, doc: Doc, loader: Loader) -> Result<Self> {
        let start = Instant::now();
        let context = &base.context;
        let ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<UniformBufferObject>() as _,
        )?;
        let skybox = SkyboxResource::new(context, Skybox::default().path())?;
        let inner = GltfViewerInner::new(base, doc, &ubo_buffer, &skybox)?;
        info!("Uploading to GPU: {}", start.elapsed().as_secs());
        Ok(GltfViewer {
            ubo_buffer,
            total_number_of_samples: 0,
            old_camera: None,
            prev_gui_state: None,
            clock: Instant::now(),
            last_update: Instant::now(),
            loader,
            skybox,
            inner: vec![inner],
        })
    }
}

impl App for GltfViewer {
    type Gui = Gui;

    fn new(base: &BaseApp<Self>) -> Result<Self> {
        use clap::Parser;
        let args = Args::parse();
        let scene = if args.file.is_empty() {
            Default::default()
        } else {
            Scene::DragAndDrop(args.file)
        };
        Self::new_with_scene(base, scene, Loader::new())
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _image_index: usize,
        frame_stats: &FrameStats,
    ) -> Result<()> {
        self.state_change(base, gui)?;
        let view = base.camera.view_matrix()
            * if gui.scale > 0. {
                gui.scale
            } else {
                1. / (gui.scale.abs() + 1.)
            };
        let inverted_view = view.try_inverse().expect("Should be invertible");

        let proj = base.camera.projection_matrix();
        let inverted_proj = proj.try_inverse().expect("Should be invertible");
        let number_of_samples =
            gui.get_number_of_samples(self.total_number_of_samples, frame_stats.frame_time);
        // println!("nums {} total: {}", number_of_samples, self.total_number_of_samples);
        if !gui.acc() {
            self.reset_samples();
        }
        self.total_number_of_samples += number_of_samples;

        let ubo = UniformBufferObject {
            model_view: view,
            projection: proj,
            model_view_inverse: inverted_view,
            projection_inverse: inverted_proj,
            aperture: gui.aperture,
            focus_distance: gui.focus_distance,
            fov_angle: 1.0,
            orthographic_fov_dis: gui.orthographic_fov_dis,
            heatmap_scale: gui.map_scale,
            total_number_of_samples: self.total_number_of_samples,
            number_of_samples,
            number_of_bounces: gui.get_bounce(),
            random_seed: 3,
            has_sky: gui.sky.into(),
            mapping: gui.mapping as _,
            antialiasing: gui.antialiasing.into(),
            frame_count: frame_stats.frame_count,
            debug: gui.debug,
            fully_opaque: self.get_inner_ref().doc.geo_builder.fully_opaque().into(),
            exposure: gui.exposure,
            tone_mapping_mode: gui.selected_tone_map_mode as _,
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
        let inner = self.get_inner_ref();
        let static_set = &inner.descriptor_res.static_set;
        let dynamic_set = &inner.descriptor_res.dynamic_sets[image_index];

        buffer.bind_rt_pipeline(&inner.pipeline_res.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            &inner.pipeline_res.pipeline_layout,
            0,
            &[static_set, dynamic_set],
        );

        buffer.trace_rays(
            &inner.sbt,
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
                let set = &self.get_inner_ref().descriptor_res.dynamic_sets[index];

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
                    let set = &self.get_inner_ref().descriptor_res.dynamic_sets[i];

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

    fn drag_and_drop(&mut self, path: PathBuf, gui: &mut Gui) {
        let path = path.into_os_string().into_string().unwrap_or("".to_owned());
        gui.scene = Scene::DragAndDrop(path.clone());
        self.loader.load(path);
    }

    fn state_change(
        &mut self,
        base: &mut BaseApp<Self>,
        gui_state: &mut <Self as App>::Gui,
    ) -> Result<()> {
        if let Some(doc) = self.loader.get_model() {
            base.wait_for_gpu()?;
            self.inner.clear();
            self.inner.push(GltfViewerInner::new(
                base,
                doc,
                &self.ubo_buffer,
                &self.skybox,
            )?);
            self.reset_samples();
        }
        if self.old_camera.is_none() {
            self.old_camera = Some(base.camera);
        }
        if self.prev_gui_state.is_none() {
            self.prev_gui_state = Some(gui_state.clone());
        }

        if self.old_camera.filter(|x| *x != base.camera).is_some() {
            self.old_camera = Some(base.camera);
            self.reset_samples();
        }

        if let Some(old_state) = self.prev_gui_state.clone().filter(|x| x != gui_state) {
            if old_state.scene != gui_state.scene {
                self.loader.load(gui_state.scene.path());
                // *self = Self::new_with_scene(base, gui_state.scene, gui_state.skybox, self.loader.clone()).unwrap();
            }
            if old_state.skybox != gui_state.skybox {
                let skybox = SkyboxResource::new(&base.context, gui_state.skybox.path())?;
                skybox.update_desc(&self.get_inner_ref().descriptor_res.static_set, SKYBOX_BIND);
                self.skybox = skybox;
            }
            self.prev_gui_state = Some(gui_state.clone());
            self.reset_samples();
            if old_state.sun != gui_state.sun {
                let inner = self.get_inner_mut();
                inner.globals.d_lights[0] = gui_state.sun;
                inner
                    .buffers
                    .dlights_buffer
                    .copy_data_to_buffer(inner.globals.d_lights.as_slice())?;
            }
            if old_state.point_light_intensity != gui_state.point_light_intensity
                || gui_state.point_light_radius != old_state.point_light_radius
            {
                let inner = self.get_inner_mut();
                inner.globals.p_lights.iter_mut().for_each(|x| {
                    let mut new_light = LightRaw::random_light(gui_state.point_light_radius);
                    new_light.intensity = gui_state.point_light_intensity;
                    *x = new_light;
                });
                inner
                    .buffers
                    .plights_buffer
                    .copy_data_to_buffer(inner.globals.p_lights.as_slice())?;
            }
        }

        if !self.get_inner_ref().doc.static_scene() && gui_state.animation && self.need_update() {
            self.last_update = Instant::now();
            let t = self.clock.elapsed().as_secs_f32() * gui_state.animation_speed;
            self.get_inner_mut().doc.animate(t);
            let mut blas_opt = None;
            let tlas = {
                let inner = self.get_inner_ref();
                if let Some((skin, _ani)) = &inner.buffers.animation_buffers {
                    let new_skin = inner.doc.get_skins();
                    skin.copy_data_to_buffer(&new_skin)?;
                    self.compute(&base.context)?;
                    let (blas, tlas) = create_as(
                        &base.context,
                        &inner.doc,
                        &inner.buffers,
                        vk::BuildAccelerationStructureFlagsKHR::empty(),
                    )?;
                    blas_opt = Some(blas);
                    tlas
                } else {
                    create_top_as(
                        &base.context,
                        &inner.doc,
                        &inner._bottom_as,
                        vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
                        None,
                    )?
                }
            };
            if let Some(b) = blas_opt {
                self.get_inner_mut()._bottom_as = b
            };
            self.update_tlas(tlas);
        }
        Ok(())
    }
}

impl GltfViewer {
    fn get_inner_mut(&mut self) -> &mut GltfViewerInner {
        &mut self.inner[0]
    }

    fn get_inner_ref(&self) -> &GltfViewerInner {
        &self.inner[0]
    }

    fn update_tlas(&mut self, tlas: TopAS) {
        let inner = self.get_inner_mut();
        inner
            .descriptor_res
            .static_set
            .update(&[WriteDescriptorSet {
                binding: AS_BIND,
                kind: WriteDescriptorSetKind::AccelerationStructure {
                    acceleration_structure: &tlas.inner,
                },
            }]);
        inner.top_as = tlas;
    }

    fn need_update(&self) -> bool {
        self.last_update.elapsed().as_secs_f32() >= 1. / 60.
    }

    fn reset_samples(&mut self) {
        self.total_number_of_samples = 0;
    }

    fn compute(&self, context: &Context) -> Result<()> {
        let inner = self.get_inner_ref();
        if let Some(compute) = &inner.compute_unit {
            let vertex_count = inner.doc.geo_builder.vertices.len() as u32;
            compute.dispatch(context, &inner.buffers, vertex_count)?;
        }
        Ok(())
    }
}
