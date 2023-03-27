use crate::{Index, MaterialID, MeshID};
use glam::{vec4, Vec2, Vec4, Vec4Swizzles};
use gltf::mesh::Mode;
use gltf::{buffer, Semantic};

pub struct Mesh {
    index: MeshID,
    pub(crate) primitives: Vec<Primitive>,
}

#[derive(Default)]
pub struct GeoBuilder {
    pub(crate) buffers: Vec<buffer::Data>,
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<Index>,
    pub(crate) geo_counter: u32,
    // Vertex offset, indices offset, material id
    pub(crate) offsets: Vec<[u32; 3]>,
    // Vertex len, indices len
    pub len: Vec<[usize; 2]>
}

#[repr(C)]
pub struct PrimInfo {
    v_offset: u32,
    i_offset: u32,
    material_id: u32,
    _padding: u32,
}

impl PrimInfo {
    fn new(&[v_off, i_off, mat]: &[u32; 3]) -> Self {
        Self {
            v_offset: v_off,
            i_offset: i_off,
            material_id: mat,
            _padding: 0
        }
    }
}

impl GeoBuilder {
    fn next_geo_id(&mut self) -> u32 {
        let cur = self.geo_counter;
        self.geo_counter += 1;
        cur
    }

    pub fn flatten(&self) -> Vec<PrimInfo> {
        self.offsets.iter().map(PrimInfo::new).collect()
    }
}

impl Mesh {
    pub(crate) fn new(mesh: gltf::Mesh, builder: &mut GeoBuilder) -> Self {
        let index = mesh.index();
        let mut primitives = vec![];
        for primitive in mesh.primitives().filter(is_primitive_supported) {
            primitives.push(Primitive::from(primitive, builder));
        }
        Mesh { primitives, index }
    }
}

pub struct Primitive {
    pub(crate) material: MaterialID,
    pub geometry_id: u32,
}

const DEFAULT_MATERIAL_INDEX: usize = 0;
impl Primitive {
    fn from(primitive: gltf::Primitive, builder: &mut GeoBuilder) -> Self {
        let geo_id = builder.next_geo_id();

        let material = primitive.material();
        let material_index = material.index().unwrap_or(DEFAULT_MATERIAL_INDEX) as u32;

        let (vertices, indices): (Vec<Vertex>, Vec<Index>) = {
            let reader = primitive.reader(|buffer| Some(&builder.buffers[buffer.index()]));
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
                        material_index
                    }
                })
                .collect();
            (vertices, indices)
        };
        let v_offset = builder.vertices.len();
        let i_offset = builder.indices.len();
        builder.len.push([vertices.len(), indices.len()]);
        builder.vertices.extend(vertices);
        builder.indices.extend(indices);
        builder.offsets.push([v_offset as _, i_offset as _, material_index as _]);

        Primitive {
            material: material_index as usize,
            geometry_id: geo_id,
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
    pub material_index: u32,
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
