use crate::{get_name, Name, Node, Vertex};
use glam::{vec4, Mat4, Vec2, Vec4, Vec4Swizzles};
use gltf::buffer::Data;
use gltf::material::{AlphaMode, NormalTexture, OcclusionTexture};
use gltf::mesh::{Mode};
use gltf::{texture, Document, Mesh, Primitive, Semantic};
use std::collections::{HashMap};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TextureInfo {
    pub texture_index: i32,
    tex_coord: i32,
}

impl Default for TextureInfo {
    fn default() -> Self {
        Self {
            tex_coord: -1,
            texture_index: -1,
        }
    }
}

impl TextureInfo {
    fn new(info: Option<texture::Info>) -> Self {
        info.map(|t| Self {
            texture_index: t.texture().index() as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }

    fn new_normal(info: Option<NormalTexture>) -> Self {
        info.map(|t| Self {
            texture_index: t.texture().index() as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }

    fn new_occ(info: Option<OcclusionTexture>) -> Self {
        info.map(|t| Self {
            texture_index: t.texture().index() as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: Name,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: bool,

    pub base_color: [f32; 4],
    pub base_color_texture: TextureInfo,

    pub metallic_factor: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: TextureInfo,

    pub normal_texture: TextureInfo,

    pub emissive_factor: [f32; 4],
    pub emissive_texture: TextureInfo,

    pub occlusion_texture: TextureInfo,
    pub ior: f32,
}

impl<'a> From<gltf::Material<'a>> for Material {
    fn from(material: gltf::Material) -> Self {
        let pbr = material.pbr_metallic_roughness();
        let em = material.emissive_factor();
        Self {
            alpha_mode: material.alpha_mode(),
            alpha_cutoff: material.alpha_cutoff(),
            double_sided: material.double_sided(),

            base_color: pbr.base_color_factor(),
            base_color_texture: TextureInfo::new(pbr.base_color_texture()),

            metallic_factor: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            metallic_roughness_texture: TextureInfo::new(pbr.metallic_roughness_texture()),

            normal_texture: TextureInfo::new_normal(material.normal_texture()),

            emissive_factor: [em[0], em[1], em[2], 0.],
            emissive_texture: TextureInfo::new(material.emissive_texture()),

            occlusion_texture: TextureInfo::new_occ(material.occlusion_texture()),
            ior: material.ior().unwrap_or(0.),
            name: get_name(material.name()),
        }
    }
}

pub fn load_materials(doc: &Document) -> Vec<Material> {
    doc.materials().map(Material::from).collect()
}

pub fn check_extensions(doc: &Document) {
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

fn find_used_meshes<'a, 'b: 'a>(node: gltf::Node<'b>, meshes: &mut Vec<Mesh<'a>>) {
    node.mesh().map(|m| meshes.push(m));
    node.children().for_each(|c| find_used_meshes(c, meshes));
}

pub fn load_drawable_nodes(doc: &Document) -> Vec<Node> {
    check_extensions(doc);
    let scene = doc.default_scene().unwrap_or(doc.scenes().next().unwrap());
    let mut meshes = vec![];
    scene
        .nodes()
        .for_each(|node| find_used_meshes(node, &mut meshes));
    let nodes: Vec<_> = doc.nodes().collect();
    for node in scene.nodes() {
        process_node(&nodes, node.index(), Mat4::IDENTITY);
    }
    vec![]
}

fn is_primitive_supported(primitive: &Primitive) -> bool {
    primitive.get(&Semantic::Positions).is_some() && primitive.mode() == Mode::Triangles
}

fn create_normal(position: &[Vec4], indices: &[u32]) -> Vec<Vec4> {
    let mut normals = vec![Vec4::default(); indices.len()];
    for i in 0..indices.len() {
        let i0 = indices[i + 0] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;
        let p0 = position[i0];
        let p1 = position[i1];
        let p2 = position[i2];
        let v0 = (p1 - p0).xyz().normalize();
        let v1 = (p2 - p0).xyz().normalize();
        let n = v0.cross(v1).normalize();
        let n = Vec4::new(n[0], n[1], n[2], 0.);
        normals[i0] = n;
    }
    normals
}

pub fn process_mesh(meshes: &[Mesh], buffers: &[Data]) -> Vec<crate::Mesh> {
    let mut mesh_record = HashMap::<[usize; 2], usize>::new();

    let mut vertices = vec![];
    let mut indices = vec![];
    let mut meshes_new: Vec<crate::Mesh> = vec![];

    for mesh in meshes {
        for primitive in mesh.primitives().filter(is_primitive_supported) {
            let og_index = [mesh.index(), primitive.index()];

            let material = primitive.material().into();

            if let Some(&m) = mesh_record.get(&og_index) {
                let mut new_mesh = meshes_new[m].clone();
                new_mesh.material = material;
                meshes_new.push(new_mesh);
                continue;
            }

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            // vertices
            let vertex_reader = reader.read_positions().unwrap();
            let vertex_offset = vertices.len() as _;
            let vertex_count = vertex_reader.len() as _;
            let vertex_data: Vec<_> = vertex_reader.map(|p| vec4(p[0], p[1], p[2], 0.)).collect();

            let index_offset = indices.len() as _;
            let index_count = if let Some(index_reader) = reader.read_indices() {
                let index_reader = index_reader.into_u32();
                let len = index_reader.len() as _;
                index_reader.for_each(|i| indices.push(i));
                len
            } else {
                // Create index
                (0..vertex_count).for_each(|i| indices.push(i));
                vertex_count as _
            };

            let normals = if let Some(rn) = reader.read_normals() {
                rn.map(|n| vec4(n[0], n[1], n[2], 0.0)).collect()
            } else {
                log::warn!("Creating normals");
                create_normal(&vertex_data, &indices[index_offset..])
            };

            let colors = reader
                .read_colors(0)
                .map(|reader| reader.into_rgba_f32().map(Vec4::from).collect::<Vec<_>>());

            let uvs = reader
                .read_tex_coords(0)
                .map(|reader| reader.into_f32().map(Vec2::from).collect::<Vec<_>>());

            vertex_data
                .into_iter()
                .enumerate()
                .for_each(|(index, position)| {
                    let normal = normals[index];
                    let color = colors.as_ref().map_or(Vec4::ONE, |colors| colors[index]);
                    let uvs = uvs.as_ref().map_or(Vec2::ZERO, |uvs| uvs[index]);
                    vertices.push(Vertex {
                        position,
                        normal,
                        color,
                        uvs,
                    });
                });
            let new_mesh = crate::Mesh {
                vertex_offset,
                vertex_count,
                index_offset: index_offset as u32,
                index_count,
                material,
                name: None,
            };

            mesh_record.insert(og_index, meshes_new.len());
            meshes_new.push(new_mesh);
        }
    }
    meshes_new
}

pub fn process_node(nodes: &[gltf::Node], node_index: usize, parent_matrix: Mat4) {
    let node = &nodes[node_index];
    let local_transform = node.transform().matrix();
    let _world_transform = parent_matrix * Mat4::from_cols_array_2d(&local_transform);

    if let Some(_mesh) = node.mesh() {
    } else if let Some(_camera) = node.camera() {
    } else if let Some(_light) = node.light() {
    }
    // let mut vertices = vec![];
    // let mut indices = vec![];
}
