pub use anyhow;
pub use nalgebra::{self as na};
pub use vulkan;

pub mod camera;
pub mod types;
use anyhow::Result;
use ash::vk::{self};
use camera::{Camera, Controls};
use gpu_allocator::MemoryLocation;
use gui::{
    imgui::{DrawData, Ui},
    imgui_rs_vulkan_renderer::Renderer,
    GuiContext,
};
use log;
use pretty_env_logger;
pub use resource_manager::load_spv;

use crate::types::Vec3;
use nalgebra::Point3;
use std::sync::Arc;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};
use std::path::PathBuf;
use vulkan::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const IN_FLIGHT_FRAMES: u32 = 2;

pub struct BaseApp<B: App> {
    phantom: PhantomData<B>,
    raytracing_enabled: bool,
    pub swapchain: Swapchain,
    pub command_pool: CommandPool,
    pub storage_images: Vec<ImageAndView>,
    pub acc_images: Vec<ImageAndView>,
    command_buffers: Vec<CommandBuffer>,
    in_flight_frames: InFlightFrames,
    pub context: Arc<Context>,
    pub camera: Camera,
    stats_display_mode: StatsDisplayMode,
}

pub trait App: Sized {
    type Gui: Gui;

    fn new(base: &BaseApp<Self>) -> Result<Self>;

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        gui: &mut Self::Gui,
        image_index: usize,
        frame_stats: &FrameStats,
    ) -> Result<()>;

    fn record_raytracing_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        // prevents reports of unused parameters without needing to use #[allow]
        let _ = base;
        let _ = buffer;
        let _ = image_index;

        Ok(())
    }

    fn record_raster_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        // prevents reports of unused parameters without needing to use #[allow]
        let _ = base;
        let _ = buffer;
        let _ = image_index;

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()>;
    fn state_change(
        &mut self,
        _base: &mut BaseApp<Self>,
        _gui_state: &mut Self::Gui,
    ) -> Result<()> {
        Ok(())
    }
    fn drag_and_drop(&mut self, path: PathBuf, gui: &mut Self::Gui);
}

pub trait Gui: Sized + Clone {
    fn new() -> Result<Self>;

    fn build(&mut self, ui: &Ui);
}

impl Gui for () {
    fn new() -> Result<Self> {
        Ok(())
    }

    fn build(&mut self, _ui: &Ui) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsDisplayMode {
    None,
    Basic,
    Full,
}

impl StatsDisplayMode {
    fn next(self) -> Self {
        match self {
            Self::None => Self::Basic,
            Self::Basic => Self::Full,
            Self::Full => Self::None,
        }
    }
}

pub fn run<A: App + 'static>(
    app_name: &str,
    width: u32,
    height: u32,
    enable_raytracing: bool,
) -> Result<()> {
    pretty_env_logger::init();
    let (window, event_loop) = create_window(app_name, width, height);
    let mut base_app = BaseApp::new(&window, app_name, enable_raytracing)?;
    let mut ui = Gui::new()?;
    let mut app = A::new(&mut base_app)?;
    let mut gui_context = GuiContext::new(
        &base_app.context,
        &base_app.context.command_pool,
        base_app.swapchain.format,
        &window,
        IN_FLIGHT_FRAMES as _,
    )?;

    let mut controls = Controls::default();
    let mut is_swapchain_dirty = false;
    let mut last_frame = Instant::now();
    let mut frame_stats = FrameStats::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let app = &mut app; // Make sure it is dropped before base_app

        gui_context.handle_event(&window, &event);
        controls = controls.handle_event(&event);

        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                let frame_time = now - last_frame;
                gui_context.update_delta_time(frame_time);
                last_frame = now;

                frame_stats.set_frame_time(frame_time);

                controls = controls.reset();
            }
            // On resize
            Event::WindowEvent {
                event: WindowEvent::Resized(..),
                ..
            } => {
                log::debug!("Window has been resized");
                is_swapchain_dirty = true;
            }
            // Draw
            Event::MainEventsCleared => {
                if is_swapchain_dirty {
                    let dim = window.inner_size();
                    if dim.width > 0 && dim.height > 0 {
                        base_app
                            .recreate_swapchain(dim.width, dim.height)
                            .expect("Failed to recreate swapchain");
                        app.on_recreate_swapchain(&base_app)
                            .expect("Error on recreate swapchain callback");
                    } else {
                        return;
                    }
                }

                base_app.camera = base_app.camera.update(&controls, frame_stats.frame_time);

                is_swapchain_dirty = base_app
                    .draw(&window, app, &mut gui_context, &mut ui, &mut frame_stats)
                    .expect("Failed to tick");
            }
            // Keyboard
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(key_code),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if key_code == VirtualKeyCode::R && state == ElementState::Pressed {
                    base_app.toggle_stats();
                }
            }
            Event::WindowEvent {
               event: WindowEvent::DroppedFile(path),
                ..
            } => {
                app.drag_and_drop(path, &mut ui)
            }
            // Mouse
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                if button == MouseButton::Right {
                    if state == ElementState::Pressed {
                        window.set_cursor_visible(false);
                    } else {
                        window.set_cursor_visible(true);
                    }
                }
            }
            // Exit app on request to close window
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            // Wait for gpu to finish pending work before closing app
            Event::LoopDestroyed => base_app
                .wait_for_gpu()
                .expect("Failed to wait for gpu to finish work"),
            _ => (),
        }
    });
}

fn create_window(app_name: &str, width: u32, height: u32) -> (Window, EventLoop<()>) {
    log::debug!("Creating window and event loop");
    let events_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(app_name)
        .with_inner_size(PhysicalSize::new(width, height))
        .with_resizable(true)
        .build(&events_loop)
        .unwrap();

    (window, events_loop)
}

impl<B: App> BaseApp<B> {
    fn new(window: &Window, app_name: &str, enable_raytracing: bool) -> Result<Self> {
        log::info!("Create application: {}", app_name);

        // Vulkan context
        let mut required_extensions = vec!["VK_KHR_swapchain", "VK_KHR_shader_clock"];
        if enable_raytracing {
            required_extensions.push("VK_KHR_ray_tracing_pipeline");
            required_extensions.push("VK_KHR_acceleration_structure");
            required_extensions.push("VK_KHR_deferred_host_operations");
        }

        let context = ContextBuilder::new(window)
            .vulkan_version(VERSION_1_3)
            .app_name(app_name)
            .required_extensions(&required_extensions)
            .required_device_features(DeviceFeatures {
                ray_tracing_pipeline: enable_raytracing,
                acceleration_structure: enable_raytracing,
                runtime_descriptor_array: enable_raytracing,
                buffer_device_address: enable_raytracing,
                dynamic_rendering: true,
                synchronization2: true,
            })
            .with_raytracing_context(enable_raytracing)
            .build()?;

        let command_pool = context.create_command_pool(
            context.graphics_queue_family,
            Some(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
        )?;

        let swapchain = Swapchain::new(
            &context,
            window.inner_size().width,
            window.inner_size().height,
        )?;

        let storage_images = if enable_raytracing {
            create_storage_images(
                &context,
                swapchain.format,
                swapchain.extent,
                swapchain.images.len(),
            )?
        } else {
            vec![]
        };

        let acc_images = if enable_raytracing {
            create_storage_images(
                &context,
                vk::Format::R32G32B32A32_SFLOAT,
                swapchain.extent,
                1,
            )?
        } else {
            vec![]
        };

        let command_buffers = create_command_buffers(&command_pool, &swapchain)?;

        let in_flight_frames = InFlightFrames::new(&context, IN_FLIGHT_FRAMES)?;

        let camera = Camera::new(
            Point3::from([0., 0., 1.]),
            -Vec3::z(),
            60.0,
            window.inner_size().width as f32 / window.inner_size().height as f32,
            0.1,
            10.0,
        );

        Ok(Self {
            phantom: PhantomData,
            raytracing_enabled: enable_raytracing,
            context,
            command_pool,
            swapchain,
            storage_images,
            acc_images,
            command_buffers,
            in_flight_frames,
            camera,
            stats_display_mode: StatsDisplayMode::Basic,
        })
    }

    fn recreate_swapchain(&mut self, width: u32, height: u32) -> Result<()> {
        log::debug!("Recreating the swapchain");

        self.wait_for_gpu()?;

        // Swapchain and dependent resources
        self.swapchain.resize(&self.context, width, height)?;

        // Recreate storage image for RT and update descriptor set
        let storage_images = create_storage_images(
            &mut self.context,
            self.swapchain.format,
            self.swapchain.extent,
            self.swapchain.images.len(),
        )?;

        let acc_images = create_storage_images(
            &mut self.context,
            vk::Format::R32G32B32A32_SFLOAT,
            self.swapchain.extent,
            self.swapchain.images.len(),
        )?;

        let _ = std::mem::replace(&mut self.storage_images, storage_images);
        let _ = std::mem::replace(&mut self.acc_images, acc_images);

        // Update camera aspect ration
        self.camera.aspect_ratio = width as f32 / height as f32;

        Ok(())
    }

    pub fn wait_for_gpu(&self) -> Result<()> {
        self.context.device_wait_idle()
    }

    fn draw(
        &mut self,
        window: &Window,
        base_app: &mut B,
        gui_context: &mut GuiContext,
        gui: &mut B::Gui,
        frame_stats: &mut FrameStats,
    ) -> Result<bool> {
        // Drawing the frame
        self.in_flight_frames.next();
        self.in_flight_frames.fence().wait(None)?;

        // Can't get for gpu time on the first frames or vkGetQueryPoolResults gets stuck
        // due to VK_QUERY_RESULT_WAIT_BIT
        let gpu_time = (frame_stats.total_frame_count >= IN_FLIGHT_FRAMES)
            .then(|| self.in_flight_frames.gpu_frame_time_ms())
            .transpose()?
            .unwrap_or_default();
        frame_stats.set_gpu_time_time(gpu_time);
        frame_stats.tick();

        let next_image_result = self
            .swapchain
            .acquire_next_image(u64::MAX, self.in_flight_frames.image_available_semaphore());
        let image_index = match next_image_result {
            Ok(AcquiredImage { index, .. }) => index as usize,
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Error while acquiring next image. Cause: {}", err),
            },
        };
        self.in_flight_frames.fence().reset()?;

        // Generate UI
        gui_context
            .platform
            .prepare_frame(gui_context.imgui.io_mut(), window)?;
        let ui = gui_context.imgui.frame();

        gui.build(&ui);
        self.build_perf_ui(&ui, frame_stats, window.scale_factor() as _);

        gui_context.platform.prepare_render(&ui, window);
        let draw_data = gui_context.imgui.render();

        base_app.update(self, gui, image_index, frame_stats)?;

        let command_buffer = &self.command_buffers[image_index];

        self.record_command_buffer(
            command_buffer,
            image_index,
            base_app,
            &mut gui_context.renderer,
            draw_data,
        )?;

        self.context.graphics_queue.submit(
            command_buffer,
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.image_available_semaphore(),
                stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }),
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.render_finished_semaphore(),
                stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }),
            self.in_flight_frames.fence(),
        )?;

        let signal_semaphores = [self.in_flight_frames.render_finished_semaphore()];
        let present_result = self.swapchain.queue_present(
            image_index as _,
            &signal_semaphores,
            &self.context.present_queue,
        );
        match present_result {
            Ok(true) => return Ok(true),
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Failed to present queue. Cause: {}", err),
            },
            _ => {}
        }

        Ok(false)
    }

    fn build_perf_ui(&self, ui: &Ui, frame_stats: &mut FrameStats, scale: f32) {
        let width = self.swapchain.extent.width as f32 / scale;
        let height = self.swapchain.extent.height as f32 / scale;
        // println!("width{} height{}", width, height);
        if matches!(
            self.stats_display_mode,
            StatsDisplayMode::Basic | StatsDisplayMode::Full
        ) {
            ui.window("Frame stats")
                .focus_on_appearing(false)
                .no_decoration()
                .bg_alpha(0.5)
                .position([width * 0.7, 5.0], gui::imgui::Condition::Always)
                .size([160.0, 140.0], gui::imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("Framerate");
                    ui.label_text("fps", frame_stats.fps_counter.to_string());
                    ui.text("Frametimes");
                    ui.label_text("all", format!("{:?}", frame_stats.frame_time));
                    ui.label_text("cpu", format!("{:?}", frame_stats.cpu_time));
                    ui.label_text("gpu", format!("{:?}", frame_stats.gpu_time));
                });
        }

        if matches!(self.stats_display_mode, StatsDisplayMode::Full) {
            let graph_size = [width - 80.0, 40.0];
            const SCALE_MIN: f32 = 0.0;
            const SCALE_MAX: f32 = 17.0;

            ui.window("Frametime graphs")
                .focus_on_appearing(false)
                .no_decoration()
                .bg_alpha(0.5)
                .position([5.0, height * 0.7], gui::imgui::Condition::Always)
                .size([width - 10.0, 140.0], gui::imgui::Condition::Always)
                .build(|| {
                    ui.plot_lines("Frame", &frame_stats.frame_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                    ui.plot_lines("CPU", &frame_stats.cpu_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                    ui.plot_lines("GPU", &frame_stats.gpu_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                });
        }
    }

    fn record_command_buffer(
        &self,
        buffer: &CommandBuffer,
        image_index: usize,
        base_app: &B,
        gui_renderer: &mut Renderer,
        draw_data: &DrawData,
    ) -> Result<()> {
        let swapchain_image = &self.swapchain.images[image_index];
        let swapchain_image_view = &self.swapchain.views[image_index];

        buffer.reset()?;

        buffer.begin(None)?;

        buffer.reset_all_timestamp_queries_from_pool(self.in_flight_frames.timing_query_pool());

        buffer.write_timestamp(
            vk::PipelineStageFlags2::NONE,
            self.in_flight_frames.timing_query_pool(),
            0,
        );

        if self.raytracing_enabled {
            let storage_image = &self.storage_images[image_index].image;
            let _acc_image = &self.acc_images[0].image;
            // base_app.compute(&self.context, buffer)?;
            base_app.record_raytracing_commands(self, buffer, image_index)?;

            // Copy ray tracing result into swapchain
            buffer.pipeline_image_barriers(&[
                ImageBarrier {
                    image: swapchain_image,
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::NONE,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::NONE,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::GENERAL,
                    new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::SHADER_WRITE,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    src_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
            ]);

            buffer.copy_image(
                storage_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            buffer.pipeline_image_barriers(&[
                ImageBarrier {
                    image: swapchain_image,
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout: vk::ImageLayout::GENERAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    dst_access_mask: vk::AccessFlags2::NONE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                },
            ]);
        }

        if !self.raytracing_enabled {
            buffer.pipeline_image_barriers(&[ImageBarrier {
                image: swapchain_image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }]);
        }

        // Rasterization
        base_app.record_raster_commands(self, buffer, image_index)?;

        // UI
        buffer.begin_rendering(
            swapchain_image_view,
            self.swapchain.extent,
            vk::AttachmentLoadOp::DONT_CARE,
            None,
        );
        let [w, h] = draw_data.display_size;
        if w > f32::EPSILON && h > f32::EPSILON {
            gui_renderer.cmd_draw(buffer.inner, draw_data)?;
        }
        buffer.end_rendering();

        buffer.pipeline_image_barriers(&[ImageBarrier {
            image: swapchain_image,
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_READ,
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        }]);

        buffer.write_timestamp(
            vk::PipelineStageFlags2::ALL_COMMANDS,
            self.in_flight_frames.timing_query_pool(),
            1,
        );

        buffer.end()?;

        Ok(())
    }

    fn toggle_stats(&mut self) {
        self.stats_display_mode = self.stats_display_mode.next();
    }
}

fn create_storage_images(
    context: &Arc<Context>,
    format: vk::Format,
    extent: vk::Extent2D,
    count: usize,
) -> Result<Vec<ImageAndView>> {
    let mut images = Vec::with_capacity(count);

    for _ in 0..count {
        let image = context.create_image(
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            MemoryLocation::GpuOnly,
            format,
            extent.width,
            extent.height,
        )?;

        let view = image.create_image_view()?;

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::GENERAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::NONE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }]);
        })?;

        images.push(ImageAndView { image, view })
    }

    Ok(images)
}

fn create_command_buffers(pool: &CommandPool, swapchain: &Swapchain) -> Result<Vec<CommandBuffer>> {
    pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, swapchain.images.len() as _)
}

pub struct ImageAndView {
    pub view: ImageView,
    pub image: Image,
}

struct InFlightFrames {
    per_frames: Vec<PerFrame>,
    current_frame: usize,
}

struct PerFrame {
    image_available_semaphore: Semaphore,
    render_finished_semaphore: Semaphore,
    fence: Fence,
    timing_query_pool: TimestampQueryPool<2>,
}

impl InFlightFrames {
    fn new(context: &Context, frame_count: u32) -> Result<Self> {
        let sync_objects = (0..frame_count)
            .map(|_i| {
                let image_available_semaphore = context.create_semaphore()?;
                let render_finished_semaphore = context.create_semaphore()?;
                let fence = context.create_fence(Some(vk::FenceCreateFlags::SIGNALED))?;

                let timing_query_pool = context.create_timestamp_query_pool()?;

                Ok(PerFrame {
                    image_available_semaphore,
                    render_finished_semaphore,
                    fence,
                    timing_query_pool,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            per_frames: sync_objects,
            current_frame: 0,
        })
    }

    fn next(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.per_frames.len();
    }

    fn image_available_semaphore(&self) -> &Semaphore {
        &self.per_frames[self.current_frame].image_available_semaphore
    }

    fn render_finished_semaphore(&self) -> &Semaphore {
        &self.per_frames[self.current_frame].render_finished_semaphore
    }

    fn fence(&self) -> &Fence {
        &self.per_frames[self.current_frame].fence
    }

    fn timing_query_pool(&self) -> &TimestampQueryPool<2> {
        &self.per_frames[self.current_frame].timing_query_pool
    }

    fn gpu_frame_time_ms(&self) -> Result<Duration> {
        let result = self.timing_query_pool().wait_for_all_results()?;
        let time = Duration::from_nanos(result[1].saturating_sub(result[0]));

        Ok(time)
    }
}

#[derive(Debug)]
pub struct FrameStats {
    // we collect gpu timings the frame after it was computed
    // so we keep frame times for the two last frames
    previous_frame_time: Duration,
    pub frame_time: Duration,
    cpu_time: Duration,
    gpu_time: Duration,
    frame_time_ms_log: Queue<f32>,
    cpu_time_ms_log: Queue<f32>,
    gpu_time_ms_log: Queue<f32>,
    total_frame_count: u32,
    pub frame_count: u32,
    fps_counter: u32,
    timer: Duration,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            previous_frame_time: Default::default(),
            frame_time: Default::default(),
            cpu_time: Default::default(),
            gpu_time: Default::default(),
            frame_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            cpu_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            gpu_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            total_frame_count: Default::default(),
            frame_count: Default::default(),
            fps_counter: Default::default(),
            timer: Default::default(),
        }
    }
}

impl FrameStats {
    const ONE_SEC: Duration = Duration::from_secs(1);
    const MAX_LOG_SIZE: usize = 1000;

    fn tick(&mut self) {
        // compute cpu time
        self.cpu_time = self.previous_frame_time.saturating_sub(self.gpu_time);

        // push log
        self.frame_time_ms_log
            .push(self.previous_frame_time.as_millis() as _);
        self.cpu_time_ms_log.push(self.cpu_time.as_millis() as _);
        self.gpu_time_ms_log.push(self.gpu_time.as_millis() as _);

        // increment counter
        self.total_frame_count += 1;
        self.frame_count += 1;
        self.timer += self.frame_time;

        // reset counter if a sec has passed
        if self.timer > FrameStats::ONE_SEC {
            self.fps_counter = self.frame_count;
            self.frame_count = 0;
            self.timer -= FrameStats::ONE_SEC;
        }
    }

    fn set_frame_time(&mut self, frame_time: Duration) {
        self.previous_frame_time = self.frame_time;
        self.frame_time = frame_time;
    }

    fn set_gpu_time_time(&mut self, gpu_time: Duration) {
        self.gpu_time = gpu_time;
    }
}

#[derive(Debug)]
struct Queue<T>(Vec<T>, usize);

impl<T> Queue<T> {
    fn new(max_size: usize) -> Self {
        Self(Vec::with_capacity(max_size), max_size)
    }

    fn push(&mut self, value: T) {
        if self.0.len() == self.1 {
            self.0.remove(0);
        }
        self.0.push(value);
    }
}
