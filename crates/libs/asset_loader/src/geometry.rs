use crate::{Index, MaterialID, MeshID};
use glam::{vec4, Vec2, Vec4, Vec4Swizzles};
use gltf::mesh::Mode;
use gltf::{buffer, Semantic};

pub struct Mesh {
    index: MeshID,
    pub(crate) primitives: Vec<Primitive>,
}

impl Mesh {
    pub(crate) fn new(mesh: gltf::Mesh, buffers: &Vec<buffer::Data>) -> Self {
        let index = mesh.index();
        let mut primitives = vec![];
        for primitive in mesh.primitives().filter(is_primitive_supported) {
            primitives.push(Primitive::from(primitive, &buffers));
        }
        Mesh { primitives, index }
    }
}

pub struct Primitive {
    pub(crate) vertices: Vec<Vertex>,
    indices: Vec<Index>,
    pub(crate) material: MaterialID,
}
const DEFAULT_MATERIAL_INDEX: usize = 0;
impl Primitive {
    fn from(primitive: gltf::Primitive, buffers: &Vec<buffer::Data>) -> Self {
        let material = primitive.material();
        let material_index = material.index().unwrap_or(DEFAULT_MATERIAL_INDEX);

        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let pos_reader = reader.read_positions().unwrap();

        let vertex_data: Vec<_> = pos_reader.map(|p| vec4(p[0], p[1], p[2], 0.)).collect();

        let indices: Vec<Index> = if let Some(index_reader) = reader.read_indices() {
            let index_reader = index_reader.into_u32();
            index_reader.collect()
        } else {
            // Create index
            (0..vertex_data.len() as Index).collect()
        };

        let normals = if let Some(rn) = reader.read_normals() {
            rn.map(|n| vec4(n[0], n[1], n[2], 0.0)).collect()
        } else {
            log::warn!("Creating normals");
            create_geo_normal(&vertex_data, &indices)
        };

        let colors = reader
            .read_colors(0)
            .map(|reader| reader.into_rgba_f32().map(Vec4::from).collect::<Vec<_>>());

        let uvs = reader
            .read_tex_coords(0)
            .map(|reader| reader.into_f32().map(Vec2::from).collect::<Vec<_>>());

        let vertices = vertex_data
            .into_iter()
            .enumerate()
            .map(|(index, position)| {
                let normal = normals[index];
                let color = colors.as_ref().map_or(Vec4::ONE, |colors| colors[index]);
                let uvs = uvs.as_ref().map_or(Vec2::ZERO, |uvs| uvs[index]);
                Vertex {
                    position,
                    normal,
                    color,
                    uvs,
                }
            })
            .collect();

        Primitive {
            vertices,
            indices,
            material: material_index,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
    pub color: Vec4,
    pub uvs: Vec2,
}

fn create_geo_normal(position: &[Vec4], indices: &[u32]) -> Vec<Vec4> {
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

fn is_primitive_supported(primitive: &gltf::Primitive) -> bool {
    primitive.get(&Semantic::Positions).is_some() && primitive.mode() == Mode::Triangles
}
