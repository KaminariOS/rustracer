use crate::scene_graph::Doc;
use std::collections::HashMap;
use vulkan::{Buffer, Context, Image, ImageView, Sampler};

// One primitive per BLAS
struct VkMesh {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
}

impl VkMesh {}

// One node per instance
struct VkInstance {
    pub(crate) transform_buffer: Buffer,
}

struct VkGlobal {
    pub(crate) _images: Vec<Image>,
    pub(crate) views: Vec<ImageView>,
    pub(crate) samplers: Vec<Sampler>,
    pub(crate) textures: Vec<[usize; 3]>,
    pub vk_meshes: Vec<Vec<VkMesh>>,
}

pub fn create_model(context: &Context, doc: Doc) {
    for mesh in doc.meshes {}
}
