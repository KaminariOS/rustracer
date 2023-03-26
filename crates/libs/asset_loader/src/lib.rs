#[cfg(feature = "ash")]
pub mod acceleration_structures;
mod animation;
mod error;
mod geometry;
mod image;
mod material;
mod scene_graph;
mod texture;

type Name = Option<String>;
type Index = u32;

type SceneID = usize;
type NodeID = usize;
type MeshID = usize;
type MaterialID = usize;
type SamplerID = usize;

fn to_owned_string(r: &str) -> String {
    r.to_string()
}
