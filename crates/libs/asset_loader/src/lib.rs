mod aabb;
#[cfg(feature = "ash")]
pub mod acceleration_structures;
mod animation;
mod cubumap;
mod error;
mod geometry;
mod image;
pub mod light;
mod material;
mod morph;
mod scene_graph;
mod skinning;
mod texture;

#[cfg(feature = "ash")]
pub mod globals;

pub use crate::scene_graph::load_file;
pub use crate::scene_graph::Doc;
use gltf::Document;

type Name = Option<String>;
type Index = u32;

type SceneID = usize;
type NodeID = usize;
type MeshID = usize;

// fn to_owned_string(r: &str) -> String {
//     r.to_string()
// }

fn a3toa4<T: Copy>(a3: &[T], w: T) -> [T; 4] {
    [a3[0], a3[1], a3[2], w]
}

fn check_extensions(doc: &Document) {
    const SUPPORTED: [&str; 0] = [
        // "KHR_materials_ior",
        // "KHR_materials_pbrSpecularGlossiness",
        // "KHR_materials_transmission",
        // "KHR_materials_variants",
        // "KHR_materials_volume",
        // "KHR_materials_specular",
        // "KHR_texture_transform",
        // "KHR_materials_unlit"
    ];
    doc.extensions_used()
        .filter(|ext| SUPPORTED.iter().all(|s| s != ext))
        .for_each(|ext| log::error!("Extension {} is used but not supported", ext));
}

#[macro_export]
macro_rules! check_indices {
    ($expr:expr) => {
        assert!($expr.iter().enumerate().all(|(i, m)| i == m.index));
    };
}

#[macro_export]
macro_rules! get_index {
    ($expr:expr) => {
        $expr.map(|m| m.index())
    };
}

#[macro_export]
macro_rules! get_index_array {
    ($expr:expr) => {
        get_index!($expr).collect::<Vec<_>>()
    };
}

#[macro_export]
macro_rules! get_name {
    ($expr:expr) => {
        $expr.name().map(|n| n.to_string())
    };
}
