use app::anyhow::Result;
use app::vulkan::ash::vk::{self, Packed24_8};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::*;
use app::{vulkan::*, BaseApp};
use app::{App, ImageAndView};
use gltf::Vertex;
use gui::imgui::{Condition, Ui};
use std::mem::{size_of, size_of_val};
use std::time::Duration;
use app::types::*;


const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const APP_NAME: &str = "Ray traced reflections";

const MODEL_PATH: &str = "./assets/models/reflections.glb";
const MAX_DEPTH: u32 = 10;

fn main() -> Result<()> {
    app::run::<Reflections>(APP_NAME, WIDTH, HEIGHT, true)
}

struct Reflections {
    ubo_buffer: Buffer,
    _model: Model,
    _bottom_as: BottomAS,
    _top_as: TopAS,
    pipeline_res: PipelineRes,
    sbt: ShaderBindingTable,
    descriptor_res: DescriptorRes,
}

impl App for Reflections {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<SceneUBO>() as _,
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
        })
    }

    fn update(
        &mut self,
        base: &BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _image_index: usize,
        _: Duration,
    ) -> Result<()> {
        let view = base.camera.view_matrix();
        let inverted_view = view.try_inverse().expect("Should be invertible");

        let proj = base.camera.projection_matrix();
        let inverted_proj = proj.try_inverse().expect("Should be invertible");

        let light_direction = [
            gui.light.direction[0],
            gui.light.direction[1],
            gui.light.direction[2],
            0.0,
        ];
        let light_color = [
            gui.light.color[0],
            gui.light.color[1],
            gui.light.color[2],
            0.0,
        ];

        let scene_ubo = SceneUBO {
            inverted_view,
            inverted_proj,
            light_direction,
            light_color,
            max_depth: gui.max_depth,
        };

        self.ubo_buffer.copy_data_to_buffer(&[scene_ubo])?;

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

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Gui {
    light: Light,
    max_depth: u32,
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            light: Light {
                direction: [-2.0, -1.0, -2.0],
                color: [1.0; 3],
            },
            max_depth: MAX_DEPTH,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Vulkan RT")
            .size([300.0, 400.0], Condition::FirstUseEver)
            .build(|| {
                // RT controls
                ui.text_wrapped("Rays");
                let mut max_depth = self.max_depth as _;
                ui.input_int("max depth", &mut max_depth).build();
                self.max_depth = max_depth.max(1) as _;

                // Light control
                ui.text_wrapped("Light");
                ui.separator();

                ui.input_float3("direction", &mut self.light.direction)
                    .build();

                ui.color_picker3_config("color", &mut self.light.color)
                    .display_rgb(true)
                    .build();
            });
    }
}

struct Model {
    gltf: gltf::Model,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    transform_buffer: Buffer,
    images: Vec<Image>,
    views: Vec<ImageView>,
    samplers: Vec<Sampler>,
    textures: Vec<(usize, usize)>,
}

struct BottomAS {
    inner: AccelerationStructure,
    geometry_info_buffer: Buffer,
}

struct TopAS {
    inner: AccelerationStructure,
    _instance_buffer: Buffer,
}

struct PipelineRes {
    pipeline: RayTracingPipeline,
    pipeline_layout: PipelineLayout,
    static_dsl: DescriptorSetLayout,
    dynamic_dsl: DescriptorSetLayout,
}

struct DescriptorRes {
    _pool: DescriptorPool,
    static_set: DescriptorSet,
    dynamic_sets: Vec<DescriptorSet>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Light {
    direction: [f32; 3],
    color: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SceneUBO {
    inverted_view: Mat4,
    inverted_proj: Mat4,
    light_direction: [f32; 4],
    light_color: [f32; 4],
    max_depth: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GeometryInfo {
    transform: Mat4,
    base_color: [f32; 4],
    base_color_texture_index: i32,
    metallic_factor: f32,
    vertex_offset: u32,
    index_offset: u32,
}

fn create_model(context: &Context) -> Result<Model> {
    let model = gltf::load_file(MODEL_PATH)?;
    let vertices = model.vertices.as_slice();
    let indices = model.indices.as_slice();

    let vertex_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
            | vk::BufferUsageFlags::STORAGE_BUFFER,
        vertices,
    )?;

    let index_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
            | vk::BufferUsageFlags::STORAGE_BUFFER,
        indices,
    )?;

    let transforms = model
        .nodes
        .iter()
        .map(|n| {
            let transform = n.transform;
            let r0 = transform[0];
            let r1 = transform[1];
            let r2 = transform[2];
            let r3 = transform[3];

            #[rustfmt::skip]
            let matrix = [
                r0[0], r1[0], r2[0], r3[0],
                r0[1], r1[1], r2[1], r3[1],
                r0[2], r1[2], r2[2], r3[2],
            ];

            vk::TransformMatrixKHR { matrix }
        })
        .collect::<Vec<_>>();
    let transform_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &transforms,
    )?;

    let mut images = vec![];
    let mut views = vec![];

    model.images.iter().try_for_each::<_, Result<_>>(|i| {
        let width = i.width;
        let height = i.height;
        let pixels = i.pixels.as_slice();

        let staging = context.create_buffer(
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            size_of_val(pixels) as _,
        )?;

        staging.copy_data_to_buffer(pixels)?;

        let image = context.create_image(
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_SRGB,
            width,
            height,
        )?;

        context.execute_one_time_commands(|cmd| {
            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            }]);

            cmd.copy_buffer_to_image(&staging, &image, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        let view = image.create_image_view()?;

        images.push(image);
        views.push(view);

        Ok(())
    })?;

    // Dummy textures
    if images.is_empty() {
        let image = context.create_image(
            vk::ImageUsageFlags::SAMPLED,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_SRGB,
            1,
            1,
        )?;

        context.execute_one_time_commands(|cmd| {
            cmd.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        let view = image.create_image_view()?;

        images.push(image);
        views.push(view);
    }

    let mut samplers = model
        .samplers
        .iter()
        .map(|s| {
            let sampler_info = map_gltf_sampler(s);
            context.create_sampler(&sampler_info)
        })
        .collect::<Result<Vec<_>>>()?;

    // Dummy sampler
    if samplers.is_empty() {
        let sampler_info = vk::SamplerCreateInfo::builder();
        let sampler = context.create_sampler(&sampler_info)?;
        samplers.push(sampler);
    }

    let mut textures = model
        .textures
        .iter()
        .map(|t| (t.image_index, t.sampler_index))
        .collect::<Vec<_>>();

    // Dummy texture
    if textures.is_empty() {
        textures.push((0, 0));
    }

    Ok(Model {
        gltf: model,
        vertex_buffer,
        index_buffer,
        transform_buffer,
        images,
        views,
        samplers,
        textures,
    })
}

fn map_gltf_sampler<'a>(sampler: &gltf::Sampler) -> vk::SamplerCreateInfoBuilder<'a> {
    let mag_filter = match sampler.mag_filter {
        gltf::MagFilter::Linear => vk::Filter::LINEAR,
        gltf::MagFilter::Nearest => vk::Filter::NEAREST,
    };

    let min_filter = match sampler.min_filter {
        gltf::MinFilter::Linear
        | gltf::MinFilter::LinearMipmapLinear
        | gltf::MinFilter::LinearMipmapNearest => vk::Filter::LINEAR,
        gltf::MinFilter::Nearest
        | gltf::MinFilter::NearestMipmapLinear
        | gltf::MinFilter::NearestMipmapNearest => vk::Filter::NEAREST,
    };

    vk::SamplerCreateInfo::builder()
        .mag_filter(mag_filter)
        .min_filter(min_filter)
}

fn create_bottom_as(context: &mut Context, model: &Model) -> Result<BottomAS> {
    let vertex_buffer_addr = model.vertex_buffer.get_device_address();

    let index_buffer_addr = model.index_buffer.get_device_address();

    let transform_buffer_addr = model.transform_buffer.get_device_address();

    let as_geo_triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer_addr,
        })
        .vertex_stride(size_of::<Vertex>() as _)
        .max_vertex(model.gltf.vertices.len() as _)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer_addr,
        })
        .transform_data(vk::DeviceOrHostAddressConstKHR {
            device_address: transform_buffer_addr,
        })
        .build();

    let mut geometry_infos = vec![];
    let mut as_geometries = vec![];
    let mut as_ranges = vec![];
    let mut max_primitive_counts = vec![];

    for (node_index, node) in model.gltf.nodes.iter().enumerate() {
        let mesh = node.mesh;

        let primitive_count = (mesh.index_count / 3) as u32;

        geometry_infos.push(GeometryInfo {
            transform: Mat4::from_iterator( node.transform.iter().flatten().map(|x| *x)),
            base_color: mesh.material.base_color,
            base_color_texture_index: mesh
                .material
                .base_color_texture_index
                .map_or(-1, |i| i as _),
            metallic_factor: mesh.material.metallic_factor,
            vertex_offset: mesh.vertex_offset,
            index_offset: mesh.index_offset,
        });

        as_geometries.push(
            vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: as_geo_triangles_data,
                })
                .build(),
        );

        as_ranges.push(
            vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(mesh.vertex_offset)
                .primitive_count(primitive_count)
                .primitive_offset(mesh.index_offset * size_of::<u32>() as u32)
                .transform_offset((node_index * size_of::<vk::TransformMatrixKHR>()) as u32)
                .build(),
        );

        max_primitive_counts.push(primitive_count)
    }

    let geometry_info_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        &geometry_infos,
    )?;

    let inner = context.create_bottom_level_acceleration_structure(
        &as_geometries,
        &as_ranges,
        &max_primitive_counts,
    )?;

    Ok(BottomAS {
        inner,
        geometry_info_buffer,
    })
}

fn create_top_as(context: &mut Context, bottom_as: &BottomAS) -> Result<TopAS> {
    #[rustfmt::skip]
    let transform_matrix = vk::TransformMatrixKHR { matrix: [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0
    ]};

    let as_instance = vk::AccelerationStructureInstanceKHR {
        transform: transform_matrix,
        instance_custom_index_and_mask: Packed24_8::new(0, 0xFF),
        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(
            0,
            vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
        ),
        acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
            device_handle: bottom_as.inner.address,
        },
    };

    let instance_buffer = create_gpu_only_buffer_from_data(
        context,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        &[as_instance],
    )?;
    let instance_buffer_addr = instance_buffer.get_device_address();

    let as_struct_geo = vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::INSTANCES)
        .flags(vk::GeometryFlagsKHR::OPAQUE)
        .geometry(vk::AccelerationStructureGeometryDataKHR {
            instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                .array_of_pointers(false)
                .data(vk::DeviceOrHostAddressConstKHR {
                    device_address: instance_buffer_addr,
                })
                .build(),
        })
        .build();

    let as_ranges = vk::AccelerationStructureBuildRangeInfoKHR::builder()
        .first_vertex(0)
        .primitive_count(1)
        .primitive_offset(0)
        .transform_offset(0)
        .build();

    let inner =
        context.create_top_level_acceleration_structure(&[as_struct_geo], &[as_ranges], &[1])?;

    Ok(TopAS {
        inner,
        _instance_buffer: instance_buffer,
    })
}

fn create_pipeline(context: &Context, model: &Model) -> Result<PipelineRes> {
    // descriptor and pipeline layouts
    let static_layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Vertex buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        //Index buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Geometry info buffer
        vk::DescriptorSetLayoutBinding::builder()
            .binding(5)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
        // Textures
        vk::DescriptorSetLayoutBinding::builder()
            .binding(6)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.images.len() as _)
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .build(),
    ];

    let dynamic_layout_bindings = [vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
        .build()];

    let static_dsl = context.create_descriptor_set_layout(&static_layout_bindings)?;
    let dynamic_dsl = context.create_descriptor_set_layout(&dynamic_layout_bindings)?;
    let dsls = [&static_dsl, &dynamic_dsl];

    let pipeline_layout = context.create_pipeline_layout(&dsls)?;

    // Shaders
    let shaders_create_info = [
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../shaders/raygen.rgen.spv")[..],
            stage: vk::ShaderStageFlags::RAYGEN_KHR,
            group: RayTracingShaderGroup::RayGen,
        },
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../shaders/miss.rmiss.spv")[..],
            stage: vk::ShaderStageFlags::MISS_KHR,
            group: RayTracingShaderGroup::Miss,
        },
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../shaders/shadow.rmiss.spv")[..],
            stage: vk::ShaderStageFlags::MISS_KHR,
            group: RayTracingShaderGroup::Miss,
        },
        RayTracingShaderCreateInfo {
            source: &include_bytes!("../shaders/closesthit.rchit.spv")[..],
            stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            group: RayTracingShaderGroup::ClosestHit,
        },
    ];

    let pipeline_create_info = RayTracingPipelineCreateInfo {
        shaders: &shaders_create_info,
        max_ray_recursion_depth: 2,
    };

    let pipeline = context.create_ray_tracing_pipeline(&pipeline_layout, pipeline_create_info)?;

    Ok(PipelineRes {
        pipeline,
        pipeline_layout,
        static_dsl,
        dynamic_dsl,
    })
}

fn create_descriptor_sets(
    context: &Context,
    pipeline_res: &PipelineRes,
    model: &Model,
    bottom_as: &BottomAS,
    top_as: &TopAS,
    storage_imgs: &[ImageAndView],
    ubo_buffer: &Buffer,
) -> Result<DescriptorRes> {
    let set_count = storage_imgs.len() as u32;

    let pool_sizes = [
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(set_count)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(3)
            .build(),
        vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(model.images.len() as _)
            .build(),
    ];

    let pool = context.create_descriptor_pool(set_count + 1, &pool_sizes)?;

    let static_set = pool.allocate_set(&pipeline_res.static_dsl)?;
    let dynamic_sets = pool.allocate_sets(&pipeline_res.dynamic_dsl, set_count)?;

    static_set.update(&[
        WriteDescriptorSet {
            binding: 0,
            kind: WriteDescriptorSetKind::AccelerationStructure {
                acceleration_structure: &top_as.inner,
            },
        },
        WriteDescriptorSet {
            binding: 2,
            kind: WriteDescriptorSetKind::UniformBuffer { buffer: ubo_buffer },
        },
        WriteDescriptorSet {
            binding: 3,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &model.vertex_buffer,
            },
        },
        WriteDescriptorSet {
            binding: 4,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &model.index_buffer,
            },
        },
        WriteDescriptorSet {
            binding: 5,
            kind: WriteDescriptorSetKind::StorageBuffer {
                buffer: &bottom_as.geometry_info_buffer,
            },
        },
    ]);

    for (image_index, sampler_index) in model.textures.iter() {
        let view = &model.views[*image_index];
        let sampler = &model.samplers[*sampler_index];

        static_set.update(&[WriteDescriptorSet {
            binding: 6,
            kind: WriteDescriptorSetKind::CombinedImageSampler {
                view,
                sampler,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        }]);
    }

    dynamic_sets.iter().enumerate().for_each(|(index, set)| {
        set.update(&[WriteDescriptorSet {
            binding: 1,
            kind: WriteDescriptorSetKind::StorageImage {
                layout: vk::ImageLayout::GENERAL,
                view: &storage_imgs[index].view,
            },
        }]);
    });

    Ok(DescriptorRes {
        _pool: pool,
        dynamic_sets,
        static_set,
    })
}
