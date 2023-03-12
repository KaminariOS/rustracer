use app::a3toa4;
use crate::{get_name, Name};

#[derive(Debug, Clone)]
pub struct Material {
    pub base_color: [f32; 4],
    pub base_color_texture_index: Option<usize>,
    pub metallic_factor: f32,
    pub emissive_factor: [f32; 4],
    pub roughness: f32,
    pub ior: f32,
    pub name: Name
}

impl<'a> From<gltf::Material<'a>> for Material {
    fn from(material: gltf::Material) -> Self {
        let pbr = material.pbr_metallic_roughness();
        Self {
            base_color: pbr.base_color_factor(),
            base_color_texture_index: pbr.base_color_texture().map(|i| i.texture().index()),
            metallic_factor: pbr.metallic_factor(),
            emissive_factor: a3toa4(&material.emissive_factor(), 0.),
            roughness: pbr.roughness_factor(),
            ior: material.ior().unwrap_or(0.),
            name: get_name(material.name()),
        }
    }
}
