use crate::error::*;
use crate::geometry::{GeoBuilder, Mesh};
use crate::image::Image;
use crate::material::{Material, MaterialRaw};
use crate::texture::{Sampler, Texture};
use crate::{to_owned_string, MaterialID, MeshID, Name, NodeID, SamplerID, SceneID};
use glam::Mat4;
use gltf::buffer;
use gltf::image;
use gltf::{Document};
use std::collections::HashMap;
use std::iter::{once, Once};
use std::path::Path;
use gltf::khr_lights_punctual::{Kind};

macro_rules! check_indices {
    ($ident:ident) => {
        assert!($ident.iter().enumerate().all(|(i, m)| i == m.index));
    };
}

struct Light {
    index: usize,
    color: [f32; 3],
    name: Name,
    kind: Kind,
    range: f32,
}

impl<'a> From<gltf::khr_lights_punctual::Light<'a>> for Light {
    fn from(light: gltf::khr_lights_punctual::Light) -> Self {
        Self {
            index: light.index(),
            color: light.color(),
            name: light.name().map(to_owned_string),
            kind: light.kind(),
            range: light.range().unwrap_or(f32::MAX),
        }
    }
}

#[derive(Default)]
pub struct Doc {
    // Only one scene is in use
    current_scene: SceneID,
    scenes: HashMap<SceneID, Scene>,
    pub nodes: HashMap<NodeID, Node>,
    pub meshes: Vec<Mesh>,
    materials: Vec<Material>,
    pub(crate) textures: Vec<Texture>,
    animations: Vec<Animation>,
    lights: Vec<Light>,
    default_material_id: MaterialID,
    default_sampler_id: SamplerID,
    pub(crate) images: Vec<Image>,
    pub(crate) samplers: Vec<Sampler>,
    pub geo_builder: GeoBuilder,
}

impl Doc {
    pub fn get_current_scene(&self) -> &Scene {
        &self.scenes[&self.current_scene]
    }

    pub fn get_materials_raw(&self) -> Vec<MaterialRaw> {
        self.materials.iter().map(MaterialRaw::from).collect()
    }

    fn new(doc: &Document, buffers: Vec<buffer::Data>, gltf_images: Vec<image::Data>) -> Self {
        let current_scene = doc
            .default_scene()
            .unwrap_or(doc.scenes().next().expect("No scene"))
            .index();
        let scenes = HashMap::with_capacity(doc.scenes().len());
        let nodes = HashMap::with_capacity(doc.nodes().len());
        let lights: Vec<_> = doc.lights().into_iter().flat_map(|ls| ls.map(Light::from)).collect();
        check_indices!(lights);

        let mut geo_builder = GeoBuilder {
            buffers,
            ..Default::default()
        };
        let meshes: Vec<_> = doc
            .meshes()
            .map(|m| Mesh::new(m, &mut geo_builder))
            .collect();
        check_indices!(meshes);

        geo_builder.buffers = Vec::with_capacity(0);
        let animations = vec![];
        let images: Vec<_> = once(Image::default())
            .chain(
            gltf_images
            .iter()
            .map(Image::try_from)
            .map(Result::unwrap)
            .zip(doc.images())
            .map(|(mut img, info)| {
                img.update_info(info);
                img
            })
        )
            .collect::<_>();
        check_indices!(images);

        let samplers: Vec<_> = once(Sampler::default())
            .chain(doc.samplers().map(Sampler::from))
            .collect();
        check_indices!(samplers);

        let textures = once(Texture::default()).chain(doc.textures().map(Texture::from)).collect::<Vec<_>>();
        check_indices!(textures);

        let materials: Vec<_> = doc.materials().map(Material::from).collect();
        check_indices!(materials);

        Self {
            current_scene,
            scenes,
            nodes,
            meshes,
            materials,
            textures,
            default_material_id: 0,
            default_sampler_id: 0,
            images,
            animations,
            samplers,
            geo_builder,
            lights,
        }
    }

    fn load_scene(&mut self, document: &Document) {
        if !self.scenes.contains_key(&self.current_scene) {
            let scene = document.scenes().nth(self.current_scene).unwrap();
            let root_nodes: Vec<_> = scene.nodes().map(|n| n.index()).collect();
            self.scenes.insert(
                self.current_scene,
                Scene {
                    name: scene.name().map(to_owned_string),
                    root_nodes,
                },
            );

            scene
                .nodes()
                .into_iter()
                .for_each(|n| self.load_node(n, &document, Mat4::IDENTITY));
        }
    }

    fn load_node(&mut self, node: gltf::Node, document: &Document, parent_transform_cache: Mat4) {
        let index = node.index();
        assert!(!self.nodes.contains_key(&index));
        let local_transform = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world_transform_cache = parent_transform_cache * local_transform;
        let children_g: Vec<_> = node.children().collect();
        let mesh_g = node.mesh();

        let children = children_g.iter().map(|c| c.index()).collect();
        let mesh = mesh_g.as_ref().map(|m| m.index());
        let node_struct = Node {
            name: node.name().map(to_owned_string),
            children,
            mesh,
            local_transform,
            parent_transform_cache,
        };
        self.nodes.insert(index, node_struct);

        // mesh_g.map(|m| self.load_mesh(m, document));
        children_g
            .into_iter()
            .for_each(|c| self.load_node(c, document, world_transform_cache));
    }

    fn update_local_transform(&mut self, node_id: NodeID, new_local: Mat4) {
        let node = self.nodes.get_mut(&node_id).unwrap();
        node.local_transform = new_local;
        let parent = node.parent_transform_cache;
        self.update_parent_transform(node_id, parent);
    }

    fn update_parent_transform(&mut self, node_id: NodeID, new_parent: Mat4) {
        let node = self.nodes.get_mut(&node_id).unwrap();
        node.parent_transform_cache = new_parent;
        let local_transform = node.local_transform;
        node.children
            .clone()
            .into_iter()
            .for_each(|c| self.update_parent_transform(c, new_parent * local_transform));
    }
}

pub struct Scene {
    name: Name,
    pub root_nodes: Vec<NodeID>,
}

pub struct Node {
    name: Name,
    children: Vec<NodeID>,
    pub mesh: Option<MeshID>,
    local_transform: Mat4,
    parent_transform_cache: Mat4,
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

struct Animation {
    name: Name,
    mesh: MeshID,
}

impl Node {
    pub fn get_world_transform(&self) -> Mat4 {
        self.parent_transform_cache *
            self.local_transform
    }
}

// From Kajiya
fn iter_gltf_node_tree<F: FnMut(&gltf::scene::Node, Mat4)>(
    node: &gltf::scene::Node,
    xform: Mat4,
    f: &mut F,
) {
    let node_xform = Mat4::from_cols_array_2d(&node.transform().matrix());
    let xform = xform * node_xform;

    f(node, xform);
    for child in node.children() {
        iter_gltf_node_tree(&child, xform, f);
    }
}

fn check_extensions(doc: &Document) {
    const SUPPORTED: [&str; 1] = [
        "KHR_materials_ior",
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

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Doc> {
    let (document, buffers, gltf_images) =
        gltf::import(resource_manager::load_model(path)).map_err(|e| Error::Load(e.to_string()))?;
    check_extensions(&document);
    let mut doc = Doc::new(&document, buffers, gltf_images);
    doc.load_scene(&document);

    Ok(doc)
}

#[test]
fn test() {
    let doc = load_file("type59.gltf").expect("TODO: panic message");
    for (i, image) in doc.images.iter().enumerate() {
        assert_eq!(image.index, i);
        println!("Image: {} index:{} {:?}", i, image.index, image.source);
    }

    println!();
    for (i, mat) in doc.materials.iter().enumerate() {
        println!("Mat {} {} {:?}", i, mat.index, mat.name);
        assert_eq!(i, mat.index);
    }

    println!();

    for (i, texture) in doc.textures.iter().enumerate() {
        println!(
            "tex {} tex_index:{} img_index: {} {:?}",
            i, texture.index, texture.image_index, texture.name
        );
        assert_eq!(i, texture.index);
    }

    println!();

    for (i, sampler) in doc.samplers.iter().enumerate() {
        println!("sam {} {:?}", i, sampler.index);
        assert_eq!(i, sampler.index);
    }

    println!();

    let mut nodes: Vec<_> = doc.nodes.iter().collect();
    nodes.sort_by(|a, b| a.0.cmp(b.0));
    for (i, node) in nodes {
        println!(
            "Node {} {:?} mesh:{:?} children:{:?}",
            i, node.name, node.mesh, node.children
        );
    }
    println!();

    for (i, mesh) in doc.meshes.iter().enumerate() {
        println!("mesh {}; primitives: {}", i, mesh.primitives.len());
        for primitive in mesh.primitives.iter() {
            println!(
                "   primitive: {}; geo_id: {}",
                primitive.material, primitive.geometry_id
            );
        }
    }
}
