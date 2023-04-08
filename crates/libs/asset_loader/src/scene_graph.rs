use crate::error::*;
use crate::geometry::{GeoBuilder, Mesh};
use crate::image::{Image, process_images_unified};
use crate::material::{find_linear_textures, Material, MaterialRaw};
use crate::texture::{Sampler, Texture};
use crate::{ MeshID, Name, NodeID, SceneID, check_extensions, check_indices, get_index, get_name};
use glam::{Mat4, Vec4};
use gltf::buffer;
use gltf::image;
use gltf::{Document};

use std::iter::once;
use std::path::Path;
use std::time::Instant;
use gltf::scene::Transform;
use log::{info};
use crate::animation::{Animation, PropertyOutput};
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
    pub animations: Vec<Animation>,
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
                dlights.push(LightRaw {
                    ..Default::default()
            });
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
        let scenes: Vec<_> = doc.scenes().map(Scene::from).collect();
        check_indices!(scenes);
        let nodes: Vec<_> = doc.nodes().map(Node::from).collect();
        check_indices!(nodes);
        let lights: Vec<_> = doc.lights().into_iter().flat_map(|ls| ls.map(Light::from)).collect();
        check_indices!(lights);
        report_lights(&lights);

        let materials: Vec<_> = doc.materials().map(Material::from).collect();
        check_indices!(materials);

        let mut geo_builder = GeoBuilder::new(buffers,
                                              materials.iter().map(|m| m.has_normal_texture()).collect()
        );

        let animations: Vec<_> = doc.animations().map(|a| Animation::new(a, &geo_builder) ).collect();
        check_indices!(animations);

        let now = Instant::now();
        let meshes: Vec<_> = doc
            .meshes()
            .map(|m| Mesh::new(m, &mut geo_builder))
            .collect();
        geo_builder.buffers = Vec::with_capacity(0);
        check_indices!(meshes);
        info!("Finish processing meshes, time:{}s", now.elapsed().as_secs());

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

    fn load_scene(&mut self, _document: &Document) {
            let scene = &self.scenes[self.current_scene];
            let root_nodes  = scene.root_nodes.clone();
            root_nodes
                .into_iter()
                .for_each(|n| self.update_parent_transform(n, Mat4::IDENTITY));
    }

    fn update_local_transform(&mut self, node_id: NodeID, new_local: Transform) {
        let node = &mut self.nodes[node_id];
        node.local_transform = new_local;
        let parent = node.parent_transform_cache;
        self.update_parent_transform(node_id, parent);
    }

    fn update_parent_transform(&mut self, node_id: NodeID, new_parent: Mat4) {
        let node = &mut self.nodes[node_id];
        node.parent_transform_cache = new_parent;
        let local_transform = node.get_local_transform();
        node.children
            .clone()
            .into_iter()
            .for_each(|c| self.update_parent_transform(c, new_parent * local_transform));
    }

    pub fn animate(&mut self, t: f32) {
        let all: Vec<_> = self.animations.iter().map(|a| &a.channels)
            .flatten()
            .map(|c| {
                let target = c.target;
                let trans = c.get_transform(t);
                (target, trans)
            }).collect();
        all.into_iter().for_each(|(target, trans)|
            {
                let transform = self.nodes[target].animate(trans);
                self.update_local_transform(target, transform);
            }
        );
    }

    pub fn static_scene(&self) -> bool {
        self.animations.is_empty()
    }
}

pub struct Scene {
    index: usize,
    name: Name,
    pub root_nodes: Vec<NodeID>,
}

impl<'a> From<gltf::Scene<'_>> for Scene {
    fn from(scene: gltf::Scene) -> Self {
        Self {
            index: scene.index(),
            name: get_name!(scene),
            root_nodes: get_index!(scene.nodes()).collect(),
        }
    }
}

pub struct Node {
    index: usize,
    name: Name,
    children: Vec<NodeID>,
    light: Option<usize>,
    pub mesh: Option<MeshID>,
    local_transform: Transform,
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
            self.get_local_transform()
    }
    pub fn get_local_transform(&self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.local_transform.clone().matrix())
    }

    pub fn animate(&self, update: PropertyOutput) -> Transform {
        let (mut t, mut r, mut s ) = self.local_transform.clone().decomposed();
        match update {
            PropertyOutput::Translation(t_new) => {t = t_new;}
            PropertyOutput::Rotation(r_new) => {r = r_new;}
            PropertyOutput::Scale(s_new) => {s = s_new;}
        }
        Transform::Decomposed {translation: t, scale: s, rotation: r}
    }
}

struct Skin {
    index: usize,
    name: Name,
    joints: Vec<NodeID>,
    // inverse_bind_matrices: Vec<Mat4>,
}

impl<'a> From<gltf::Skin<'_>> for Skin {
    fn from(skin: gltf::Skin<'_>) -> Self {
        // let reader = skin.inverse_bind_matrices();
        let joints = get_index!(skin.joints()).collect();
        skin.skeleton();
        // let reader = skin.reader();
        Self {
            index: skin.index(),
            name: get_name!(skin),
            joints
        }
    }
}

impl<'a> From<gltf::Node<'_>> for Node {
    fn from(node: gltf::Node) -> Self {
        // node.skin();
        Self {
            index: node.index(),
            name: get_name!(node),
            children: get_index!(node.children()).collect(),
            light: get_index!(node.light()),
            mesh: get_index!(node.mesh()),
            local_transform: node.transform(),
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
    if !doc.static_scene() {
        info!("Animation available.");
    }
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

    let nodes: Vec<_> = doc.nodes.iter().collect();
    for  node in nodes {
        println!(
            "Node {:?} mesh:{:?} children:{:?}",
              node.name, node.mesh, node.children
        );
    }
    println!();

    for (i, mesh) in doc.meshes.iter().enumerate() {
        println!("mesh {}; primitives: {}", i, mesh.primitives.len());
        for primitive in mesh.primitives.iter() {
            println!(
                "   primitive: {}; geo_id: {}",
                primitive.geometry_id, primitive.geometry_id
            );
        }
    }
}
