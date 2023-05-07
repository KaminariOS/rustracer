use crate::aabb::{get_aabb, Aabb};
use crate::material::Material;
use crate::{a3toa4, get_name, Index, MeshID, Name};
use glam::{vec4, UVec4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::mesh::Mode;
use gltf::{buffer, Semantic};
use log::{info, warn};
use std::collections::HashMap;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
    pub tangent: [f32; 4],
    pub color: Vec4,
    pub weights: Vec4,
    pub joints: UVec4,
    pub uv0: Vec2,
    pub uv1: Vec2,
    pub skin_index: i32,
}

#[derive(Clone)]
pub struct Mesh {
    pub(crate) index: MeshID,
    pub name: Name,
    pub(crate) primitives: Vec<Primitive>,
}

#[derive(Default)]
pub struct GeoBuilder {
    pub(crate) buffers: Vec<buffer::Data>,
    pub vertices: Vec<Vertex>,
    pub(crate) indices: Vec<Index>,
    pub(crate) geo_counter: u32,
    // Vertex offset, indices offset, material id
    pub(crate) offsets: Vec<[u32; 3]>,
    // Vertex len, indices len
    pub len: Vec<[usize; 2]>,
    pub normal_textures: Vec<bool>,
    pub opaque: Vec<bool>,
    pub material_id: Vec<usize>,
}

impl GeoBuilder {
    pub fn new(buffers: Vec<buffer::Data>, materials: &[Material]) -> Self {
        Self {
            buffers,
            normal_textures: materials.iter().map(|m| m.has_normal_texture()).collect(),
            opaque: materials.iter().map(|m| m.is_opaque()).collect(),
            ..Default::default()
        }
    }

    pub fn is_opaque(&self, geo_id: u32) -> bool {
        self.opaque[self.material_id[geo_id as usize]]
    }

    pub fn fully_opaque(&self) -> bool {
        self.opaque.iter().all(|o| *o)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PrimInfo {
    v_offset: u32,
    i_offset: u32,
    material_id: u32,
    skin_id: u32,
    // transform: Mat4
}

impl PrimInfo {
    fn new(&[v_off, i_off, mat]: &[u32; 3]) -> Self {
        Self {
            v_offset: v_off,
            i_offset: i_off,
            material_id: mat,
            skin_id: 0,
            // transform: Default::default(),
        }
    }
}

impl GeoBuilder {
    pub fn next_geo_id(&mut self, material_id: u32) -> u32 {
        let cur = self.geo_counter;
        self.geo_counter += 1;
        self.material_id.push(material_id as usize);
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
        let name = mesh.name();
        info!("Building mesh {}: {:?} ", index, name);
        for primitive in mesh.primitives().filter(is_primitive_supported) {
            primitives.push(Primitive::from(primitive, builder));
        }
        Mesh {
            primitives,
            index,
            name: get_name!(mesh),
        }
    }

    pub fn get_aabb(&self) -> Option<Aabb> {
        let aabbs: Vec<_> = self.primitives.iter().map(|p| p.aabb).collect();
        Aabb::union(&aabbs)
    }
}

#[derive(Clone)]
pub struct Primitive {
    // pub(crate) material: MaterialID,
    pub geometry_id: u32,
    mapping: HashMap<u32, usize>,
    aabb: Aabb,
}

pub const DEFAULT_MATERIAL_INDEX: usize = 0;
impl Primitive {
    fn from(primitive: gltf::Primitive, builder: &mut GeoBuilder) -> Self {
        let mapping: HashMap<_, _> = primitive
            .mappings()
            .flat_map(|m| {
                let variants = m.variants();
                let material = m.material().index().unwrap_or(DEFAULT_MATERIAL_INDEX);
                variants.iter().map(move |v| (*v, material))
            })
            .collect();

        let material = primitive.material();
        let material_index = material.index().unwrap_or(DEFAULT_MATERIAL_INDEX) as u32;
        let geo_id = builder.next_geo_id(material_index);

        if !mapping.is_empty() {
            info!("Geo id {} material variants: {:?}", geo_id, mapping);
        }

        let (vertices, indices): (Vec<Vertex>, Vec<Index>) = {
            let reader = primitive.reader(|buffer| Some(&builder.buffers[buffer.index()]));
            let pos_reader = reader.read_positions().unwrap();
            // let joints_reader = reader.read_joints(0).unwrap();
            // let joints_reader = reader.read_weights(0).unwrap();
            let _morph_targets: Vec<_> = reader
                .read_morph_targets()
                .map(|(position_d, normal_d, tangent_d)| {
                    (
                        position_d.map(|p| p.map(Vec3::from)),
                        normal_d.map(|n| n.map(Vec3::from)),
                        tangent_d.map(|t| t.map(Vec3::from)),
                    )
                })
                .collect();

            let positions: Vec<_> = pos_reader.map(|p| vec4(p[0], p[1], p[2], 0.)).collect();

            let indices: Vec<Index> = if let Some(index_reader) = reader.read_indices() {
                let index_reader = index_reader.into_u32();
                index_reader.collect()
            } else {
                // Create index
                warn!("Creating index...");
                (0..positions.len() as Index).collect()
            };

            let normals = if let Some(rn) = reader.read_normals() {
                rn.map(|n| vec4(n[0], n[1], n[2], 0.0)).collect()
            } else {
                create_geo_normal(&positions, &indices)
            };

            let uvs0 = reader
                .read_tex_coords(0)
                .map(|reader| reader.into_f32().map(Vec2::from).collect::<Vec<_>>())
                .unwrap_or(vec![Vec2::ZERO; positions.len()]);
            let uvs1 = reader
                .read_tex_coords(1)
                .map(|reader| reader.into_f32().map(Vec2::from).collect::<Vec<_>>())
                .unwrap_or(vec![Vec2::ZERO; positions.len()]);

            let (mut tangents, tangents_found) = if let Some(iter) = reader.read_tangents() {
                (iter.collect::<Vec<_>>(), true)
            } else {
                (vec![[1.0, 0.0, 0.0, 0.0]; positions.len()], false)
            };
            if !tangents_found
                && !uvs0.is_empty()
                && *builder
                    .normal_textures
                    .get(material_index as usize)
                    .unwrap_or(&false)
            {
                info!("Normal map found but tangents not found. Generating tangents...");
                mikktspace::generate_tangents(&mut TangentCalcContext {
                    indices: indices.as_slice(),
                    positions: positions.as_slice(),
                    normals: normals.as_slice(),
                    uvs: uvs0.as_slice(),
                    tangents: tangents.as_mut_slice(),
                });
            }

            let colors = reader
                .read_colors(0)
                .map(|reader| reader.into_rgba_f32().map(Vec4::from).collect::<Vec<_>>());

            let weights = reader.read_weights(0).map_or(vec![], |weights| {
                weights.into_f32().map(Vec4::from).collect()
            });
            let joints = reader.read_joints(0).map_or(vec![], |joints| {
                joints
                    .into_u16()
                    .map(|[x, y, z, w]| [u32::from(x), u32::from(y), u32::from(z), u32::from(w)])
                    .map(UVec4::from)
                    .collect()
            });
            let vertices = positions
                .into_iter()
                .enumerate()
                .map(|(index, position)| {
                    let normal = normals[index];
                    let color = colors.as_ref().map_or(Vec4::ONE, |colors| colors[index]);
                    let uv = uvs0[index];
                    let weights = *weights.get(index).unwrap_or(&Default::default());
                    let joints = *joints.get(index).unwrap_or(&Default::default());
                    Vertex {
                        position,
                        normal,
                        tangent: tangents[index],
                        color,
                        weights,
                        joints,
                        uv0: uv,
                        uv1: uvs1[index],
                        skin_index: -1,
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
        builder
            .offsets
            .push([v_offset as _, i_offset as _, material_index as _]);

        Primitive {
            // material: material_index as usize,
            geometry_id: geo_id,
            mapping,
            aabb: get_aabb(&primitive.bounding_box()),
        }
    }
}

fn create_geo_normal(position: &[Vec4], indices: &[u32]) -> Vec<Vec4> {
    warn!("Creating normals");
    let mut normals = vec![Vec4::default(); indices.len()];
    assert_eq!(indices.len() % 3, 0);
    for i in 0..indices.len() / 3 {
        let i0 = indices[3 * i + 0] as usize;
        let i1 = indices[3 * i + 1] as usize;
        let i2 = indices[3 * i + 2] as usize;
        let p0 = position[i0];
        let p1 = position[i1];
        let p2 = position[i2];
        let v0 = (p1 - p0).xyz().normalize();
        let v1 = (p2 - p0).xyz().normalize();
        let n = v0.cross(v1).normalize();
        let n = Vec4::new(n[0], n[1], n[2], 0.);
        normals[i0] = n;
        normals[i1] = n;
        normals[i2] = n;
    }
    normals
}

fn is_primitive_supported(primitive: &gltf::Primitive) -> bool {
    primitive.get(&Semantic::Positions).is_some() && primitive.mode() == Mode::Triangles
}

struct TangentCalcContext<'a> {
    indices: &'a [u32],
    positions: &'a [Vec4],
    normals: &'a [Vec4],
    uvs: &'a [Vec2],
    tangents: &'a mut [[f32; 4]],
}

impl<'a> mikktspace::Geometry for TangentCalcContext<'a> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.indices[face * 3 + vert] as usize]
            .xyz()
            .to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.indices[face * 3 + vert] as usize]
            .xyz()
            .to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs[self.indices[face * 3 + vert] as usize].to_array()
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        self.tangents[self.indices[face * 3 + vert] as usize] = tangent;
    }

    fn set_tangent(
        &mut self,
        tangent: [f32; 3],
        _bi_tangent: [f32; 3],
        _f_mag_s: f32,
        _f_mag_t: f32,
        bi_tangent_preserves_orientation: bool,
        face: usize,
        vert: usize,
    ) {
        let sign = if bi_tangent_preserves_orientation {
            -1.0
        } else {
            1.0
        };
        self.set_tangent_encoded(a3toa4(&tangent, sign), face, vert);
    }
}
