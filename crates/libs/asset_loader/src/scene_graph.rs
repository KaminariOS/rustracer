use crate::error::*;
use crate::geometry::{GeoBuilder, Mesh};
use crate::image::{process_images_unified, Image};
use crate::material::{find_linear_textures, Material, MaterialRaw};
use crate::texture::{Sampler, Texture};
use crate::{
    check_extensions, check_indices, get_index, get_index_array, get_name, MeshID, Name, NodeID,
    SceneID,
};
use glam::Mat4;
use gltf::buffer;
use gltf::image;
use gltf::Document;
use std::collections::HashMap;

use crate::aabb::Aabb;
use crate::animation::{Animation, PropertyOutput};
use crate::light::{report_lights, Light, LightRaw};
use crate::skinning::{Skin, SkinRaw};
use gltf::scene::Transform;
use log::info;
use std::iter::once;
use std::path::Path;
use std::time::Instant;

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
    pub skins: Vec<Skin>,
    pub aabb_trans: Mat4,
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
            for _ in 0..5 {
                plights.push(LightRaw::random_light(10.));
            }
        }

        if dlights.is_empty() {
            dlights.push(LightRaw {
                ..Default::default()
            });
        }
        [dlights, plights]
    }

    pub fn traverse_root_nodes<F: FnMut(&Node)>(&self, f: &mut F) {
        self.get_current_scene()
            .root_nodes
            .iter()
            .map(|&node| &self.nodes[node])
            .for_each(|node| self.iter_gltf_node_tree(node, f));
    }

    pub fn need_compute(&self) -> bool {
        // Add morph later
        !self.skins.is_empty()
    }

    // From Kajiya
    fn iter_gltf_node_tree<F: FnMut(&Node)>(&self, node: &Node, f: &mut F) {
        f(node);
        node.children
            .iter()
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
        let lights: Vec<_> = doc
            .lights()
            .into_iter()
            .flat_map(|ls| ls.map(Light::from))
            .collect();
        check_indices!(lights);
        report_lights(&lights);

        let materials: Vec<_> = doc.materials().map(Material::from).collect();
        check_indices!(materials);

        let mut geo_builder = GeoBuilder::new(buffers, &materials);

        let animations: Vec<_> = doc
            .animations()
            .map(|a| Animation::new(a, &geo_builder))
            .collect();
        check_indices!(animations);
        doc.skins();
        // doc.

        let now = Instant::now();
        let meshes: Vec<_> = doc
            .meshes()
            .map(|m| Mesh::new(m, &mut geo_builder))
            .collect();
        check_indices!(meshes);
        info!(
            "Finish processing meshes, time:{}s",
            now.elapsed().as_secs()
        );

        let linear = find_linear_textures(doc);

        let now = Instant::now();
        let images = process_images_unified(&gltf_images, &doc, &linear);
        info!(
            "Finish processing images, time:{}s",
            now.elapsed().as_secs()
        );

        let samplers: Vec<_> = once(Sampler::default())
            .chain(doc.samplers().map(Sampler::from))
            .collect();
        check_indices!(samplers);

        let textures = once(Texture::default())
            .chain(doc.textures().map(Texture::from))
            .collect::<Vec<_>>();
        check_indices!(textures);

        let skins: Vec<_> = doc
            .skins()
            .map(|s| Skin::new(s, &geo_builder.buffers))
            .collect();
        check_indices!(skins);

        geo_builder.buffers = Vec::with_capacity(0);
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
            skins,
            aabb_trans: Default::default(),
        }
    }

    fn duplicate_mesh_for_non_affine_transform(&mut self) {
        let mesh_len = self.meshes.len();
        let mut mesh_to_node = vec![HashMap::new(); mesh_len];
        let mut f = |node: &Node| {
            if let Some(mesh) = node.mesh {
                mesh_to_node[mesh].insert(node.index, node.need_compute_pass());
            }
        };
        self.traverse_root_nodes(&mut f);
        for (mesh, map) in mesh_to_node.into_iter().enumerate() {
            let (mut non_affine, affine): (Vec<_>, Vec<_>) = map.into_iter().partition(|x| x.1);
            if non_affine.is_empty() {
                continue;
            }
            if affine.is_empty() {
                let node = non_affine.pop().unwrap().0;
                let skin_index = self.nodes[node].skin.unwrap();
                for p_index in 0..self.meshes[mesh].primitives.len() {
                    let geometry_id = self.meshes[mesh].primitives[p_index].geometry_id as usize;
                    let geo_builder = &mut self.geo_builder;
                    let [vertex_offset, index_offset, _material_id] =
                        geo_builder.offsets[geometry_id];
                    let [vertex_offset, _index_offset] =
                        [vertex_offset as usize, index_offset as usize];
                    let [vertex_length, _index_length] = geo_builder.len[geometry_id];
                    let dup_vertices =
                        &mut geo_builder.vertices[vertex_offset..vertex_offset + vertex_length];
                    dup_vertices.iter_mut().for_each(|v| {
                        assert_eq!(v.skin_index, -1);
                        v.skin_index = skin_index as i32;
                    });
                }
            }
            for (node, _) in non_affine {
                self.duplicate_mesh_for_node(mesh, node);
            }
        }
    }

    fn duplicate_mesh_for_node(&mut self, mesh: usize, node: usize) {
        info!(
            "Duplicating node: {} name: {:?}",
            node, self.nodes[node].name
        );
        let new_mesh_id = self.meshes.len();
        let skin_index = self.nodes[node].skin.unwrap();
        assert_eq!(self.nodes[node].mesh.unwrap(), mesh);
        *self.nodes[node].mesh.as_mut().unwrap() = new_mesh_id;
        self.meshes.push(self.meshes[mesh].clone());

        for p_index in 0..self.meshes[new_mesh_id].primitives.len() {
            let geometry_id = self.meshes[new_mesh_id].primitives[p_index].geometry_id as usize;
            let geo_builder = &mut self.geo_builder;
            let [vertex_offset, index_offset, material_id] = geo_builder.offsets[geometry_id];
            let [vertex_offset, index_offset] = [vertex_offset as usize, index_offset as usize];
            let [vertex_length, index_length] = geo_builder.len[geometry_id];
            let new_primitive_id = geo_builder.next_geo_id(material_id);
            self.meshes[new_mesh_id].primitives[p_index].geometry_id = new_primitive_id;
            let cur_vertex_len = geo_builder.vertices.len();
            let cur_index_len = geo_builder.indices.len();
            let mut dup_vertices =
                geo_builder.vertices[vertex_offset..vertex_offset + vertex_length].to_vec();
            dup_vertices.iter_mut().for_each(|v| {
                assert_eq!(v.skin_index, -1);
                v.skin_index = skin_index as i32;
            });
            // dup_vertices.
            geo_builder.vertices.extend(dup_vertices);
            geo_builder
                .indices
                .extend(geo_builder.indices[index_offset..index_offset + index_length].to_vec());
            let [vertex_offset, index_offset] = [cur_vertex_len as u32, cur_index_len as u32];
            geo_builder.len.push([vertex_length, index_length]);
            geo_builder
                .offsets
                .push([vertex_offset, index_offset, material_id]);
        }
    }

    fn load_scene(&mut self, _document: &Document) {
        let scene = &self.scenes[self.current_scene];
        let root_nodes = scene.root_nodes.clone();
        let aabbs: Vec<_> = root_nodes
            .iter()
            .filter_map(|i| self.get_node_aabb(*i))
            .collect();
        let aabb = Aabb::union(&aabbs).unwrap();
        self.aabb_trans = aabb.get_transform();
        root_nodes
            .into_iter()
            .for_each(|n| self.update_parent_transform(n, self.aabb_trans));
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
        let all: Vec<_> = self
            .animations
            .iter()
            .map(|a| &a.channels)
            .flatten()
            .map(|c| {
                let target = c.target;
                let trans = c.get_transform(t);
                (target, trans)
            })
            .collect();
        all.into_iter().for_each(|(target, trans)| {
            let transform = self.nodes[target].animate(trans);
            self.update_local_transform(target, transform);
        });
    }

    pub fn static_scene(&self) -> bool {
        self.animations.is_empty()
    }

    pub fn get_skins(&self) -> Vec<SkinRaw> {
        let skins: Vec<_> = self
            .skins
            .iter()
            .map(|s| s.get_skin_matrices(&self.nodes))
            .collect();

        skins
    }

    pub fn get_node_aabb(&self, node: usize) -> Option<Aabb> {
        let cur = &self.nodes[node];
        let mut childs: Vec<_> = cur
            .children
            .iter()
            .filter_map(|c| self.get_node_aabb(*c))
            .collect();
        if let Some(local) = cur.mesh.and_then(|m| self.meshes[m].get_aabb()) {
            childs.push(local);
        }
        childs
            .iter_mut()
            .for_each(|aabb| *aabb = *aabb * cur.get_local_transform());
        Aabb::union(&childs)
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
            root_nodes: get_index_array!(scene.nodes()),
        }
    }
}

pub struct Node {
    index: usize,
    pub skin: Option<usize>,
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
        self.parent_transform_cache * self.get_local_transform()
    }
    pub fn get_local_transform(&self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.local_transform.clone().matrix())
    }

    pub fn animate(&self, update: PropertyOutput) -> Transform {
        let (mut t, mut r, mut s) = self.local_transform.clone().decomposed();
        match update {
            PropertyOutput::Translation(t_new) => {
                t = t_new;
            }
            PropertyOutput::Rotation(r_new) => {
                r = r_new;
            }
            PropertyOutput::Scale(s_new) => {
                s = s_new;
            }
            _ => {}
        }
        Transform::Decomposed {
            translation: t,
            scale: s,
            rotation: r,
        }
    }

    pub fn need_compute_pass(&self) -> bool {
        self.skin.is_some()
    }
}

impl<'a> From<gltf::Node<'_>> for Node {
    fn from(node: gltf::Node) -> Self {
        Self {
            index: node.index(),
            skin: get_index!(node.skin()),
            name: get_name!(node),
            children: get_index_array!(node.children()),
            light: get_index!(node.light()),
            mesh: get_index!(node.mesh()),
            local_transform: node.transform(),
            parent_transform_cache: Mat4::IDENTITY,
        }
    }
}

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Doc> {
    let now = Instant::now();
    let name = path.as_ref().to_str().unwrap_or_default().to_string();
    let (document, buffers, gltf_images) =
        gltf::import(resource_manager::load_model(path)).map_err(|e| Error::Load(e.to_string()))?;

    info!(
        "Finish loading glTF {}, time:{}s",
        name,
        now.elapsed().as_secs()
    );
    check_extensions(&document);

    let mut doc = Doc::new(&document, buffers, gltf_images);
    if !doc.skins.is_empty() {
        doc.duplicate_mesh_for_non_affine_transform();
    }
    info!("Skin length: {}", doc.skins.len());
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
    for node in nodes {
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
