use crate::error::*;
use crate::geometry::{GeoBuilder, Mesh};
use crate::image::{Image, process_images_unified};
use crate::material::{find_linear_textures, Material, MaterialRaw};
use crate::texture::{Sampler, Texture};
use crate::{to_owned_string, MeshID, Name, NodeID, SceneID, check_extensions, check_indices, get_index, get_name};
use glam::Mat4;
use gltf::buffer;
use gltf::image;
use gltf::{Document};
use std::collections::HashMap;
use std::iter::once;
use std::path::Path;
use std::time::Instant;
use log::{info};
use crate::animation::Animation;
use crate::light::{Light, LightRaw, report_lights};

#[derive(Default)]
pub struct Doc {
    // Only one scene is in use
    current_scene: SceneID,
    scenes: Vec<Scene>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    materials: Vec<Material>,
    pub(crate) textures: Vec<Texture>,
    animations: Vec<Animation>,
    lights: Vec<Light>,
    // default_material_id: MaterialID,
    // default_sampler_id: SamplerID,
    pub(crate) images: Vec<Image>,
    pub(crate) samplers: Vec<Sampler>,
    pub geo_builder: GeoBuilder,
}

impl Doc {
    pub fn get_current_scene(&self) -> &Scene {
        &self.scenes[self.current_scene]
    }

    pub fn get_materials_raw(&self) -> Vec<MaterialRaw> {
        self.materials.iter().map(MaterialRaw::from).collect()
    }

    pub fn get_lights_raw(&self) -> [Vec<LightRaw>; 2] {
        let mut dlights = Vec::new();
        let mut plights = Vec::new();
        let mut f = |node: &Node| {
            if let Some(light) = node.light.map(|l| &self.lights[l]) {
                let light = light.to_raw(node.get_world_transform());
                if light.is_dir() {
                    dlights.push(light);
                } else {
                    plights.push(light);
                }
            }
        };
        self.traverse_root_nodes(&mut f);
        if plights.is_empty() {
            plights.push(Default::default());
        }

        if dlights.is_empty() {
            dlights.push(Default::default());
        }
        [dlights, plights]
    }

    pub fn traverse_root_nodes<F: FnMut(&Node)>(&self, f:&mut F) {
        self.get_current_scene().root_nodes.iter()
            .map(|&node| &self.nodes[node])
            .for_each(|node| self.iter_gltf_node_tree(node, f));
    }

    // From Kajiya
    fn iter_gltf_node_tree<F: FnMut(&Node)>(
        &self,
        node: &Node,
        f: &mut F,
    ) {
        f(node);
        node.children.iter()
            .map(|&child| &self.nodes[child])
            .for_each(|child| self.iter_gltf_node_tree(child, f))
        }

    fn new(doc: &Document, buffers: Vec<buffer::Data>, gltf_images: Vec<image::Data>) -> Self {
        let current_scene = doc
            .default_scene()
            .unwrap_or(doc.scenes().next().expect("No scene"))
            .index();
        let scenes = doc.scenes().map(Scene::from).collect();
        let nodes = doc.nodes().map(Node::from).collect();
        let lights: Vec<_> = doc.lights().into_iter().flat_map(|ls| ls.map(Light::from)).collect();
        check_indices!(lights);
        report_lights(&lights);

        let mut geo_builder = GeoBuilder {
            buffers,
            ..Default::default()
        };

        let mut now = Instant::now();
        let meshes: Vec<_> = doc
            .meshes()
            .map(|m| Mesh::new(m, &mut geo_builder))
            .collect();
        check_indices!(meshes);
        info!("Finish processing meshes, time:{}s", now.elapsed().as_secs());

        let materials: Vec<_> = doc.materials().map(Material::from).collect();
        check_indices!(materials);

        geo_builder.buffers = Vec::with_capacity(0);
        let animations = vec![];

        let linear = find_linear_textures(&materials);

        let now = Instant::now();
        let images = process_images_unified(&gltf_images, &doc, &linear);
        info!("Finish processing images, time:{}s", now.elapsed().as_secs());

        let samplers: Vec<_> = once(Sampler::default())
            .chain(doc.samplers().map(Sampler::from))
            .collect();
        check_indices!(samplers);

        let textures = once(Texture::default()).chain(doc.textures().map(Texture::from)).collect::<Vec<_>>();
        check_indices!(textures);



        Self {
            current_scene,
            scenes,
            nodes,
            meshes,
            materials,
            textures,
            // default_material_id: 0,
            // default_sampler_id: 0,
            images,
            animations,
            samplers,
            geo_builder,
            lights,
        }
    }

    fn load_scene(&mut self, document: &Document) {
            let scene = &self.scenes[self.current_scene];
            let root_nodes  = scene.root_nodes.clone();
            root_nodes
                .into_iter()
                .for_each(|n| self.load_node(n, &document, Mat4::IDENTITY));
    }

    fn load_node(&mut self, index: usize, document: &Document, parent_transform_cache: Mat4) {
        let node = &mut self.nodes[index];
        let world_transform_cache = parent_transform_cache * node.local_transform;
        node.parent_transform_cache = parent_transform_cache;
        node.children.clone()
            .into_iter()
            .for_each(|c| self.load_node(c, document, world_transform_cache));
    }

    fn update_local_transform(&mut self, node_id: NodeID, new_local: Mat4) {
        let node = &mut self.nodes[node_id];
        node.local_transform = new_local;
        let parent = node.parent_transform_cache;
        self.update_parent_transform(node_id, parent);
    }

    fn update_parent_transform(&mut self, node_id: NodeID, new_parent: Mat4) {
        let node = &mut self.nodes[node_id];
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

impl<'a> From<gltf::Scene<'_>> for Scene {
    fn from(scene: gltf::Scene) -> Self {
        Self {
            name: get_name!(scene),
            root_nodes: get_index!(scene.nodes()).collect(),
        }
    }
}

pub struct Node {
    name: Name,
    children: Vec<NodeID>,
    light: Option<usize>,
    pub mesh: Option<MeshID>,
    local_transform: Mat4,
    parent_transform_cache: Mat4,
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}



impl Node {
    pub fn get_world_transform(&self) -> Mat4 {
        self.parent_transform_cache *
            self.local_transform
    }
}

impl<'a> From<gltf::Node<'_>> for Node {
    fn from(node: gltf::Node) -> Self {
        Self {
            name: get_name!(node),
            children: get_index!(node.children()).collect(),
            light: get_index!(node.light()),
            mesh: get_index!(node.mesh()),
            local_transform: Mat4::from_cols_array_2d(&node.transform().matrix()),
            parent_transform_cache: Mat4::IDENTITY,
        }
    }
}



pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Doc> {
    let now = Instant::now();
    info!("Start loading glTF");
    let (document, buffers, gltf_images) =
        gltf::import(resource_manager::load_model(path)).map_err(|e| Error::Load(e.to_string()))?;

    info!("Finish loading glTF, time:{}s", now.elapsed().as_secs());
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
