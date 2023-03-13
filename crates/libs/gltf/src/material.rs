use gltf::material::NormalTexture;
use gltf::texture;
use app::a3toa4;
use crate::{get_name, Name};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TextureInfo {
    texture_index: i32,
    tex_coord: i32
}

impl Default for TextureInfo {
    fn default() -> Self {
       Self {tex_coord: -1, texture_index: -1}
    }
}

impl TextureInfo {
    fn new(info: Option<texture::Info>) -> Self {
        info.map(|t| Self {texture_index: t.texture().index() as _, tex_coord: t.tex_coord() as _})
            .unwrap_or(Self::default())
    }

    fn new_normal(info: Option<NormalTexture>) -> Self {
        info.map(|t| Self {texture_index: t.texture().index() as _, tex_coord: t.tex_coord() as _})
            .unwrap_or(Self::default())
    }
}


#[derive(Debug, Clone)]
pub struct Material {
    pub base_color: [f32; 4],
    pub base_color_texture: TextureInfo,

    pub metallic_factor: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: TextureInfo,

    pub normal_texture: TextureInfo,

    pub emissive_factor: [f32; 4],
    pub emissive_texture: TextureInfo,
    pub ior: f32,
    pub name: Name
}

impl<'a> From<gltf::Material<'a>> for Material {
    fn from(material: gltf::Material) -> Self {
        let pbr = material.pbr_metallic_roughness();
        Self {
            base_color: pbr.base_color_factor(),
            base_color_texture: TextureInfo::new(pbr.base_color_texture()),

            metallic_factor: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            metallic_roughness_texture: TextureInfo::new(pbr.metallic_roughness_texture()),

            normal_texture: TextureInfo::new_normal(material.normal_texture()),

            emissive_factor: a3toa4(&material.emissive_factor(), 0.),
            emissive_texture: TextureInfo::new(material.emissive_texture()),
            ior: material.ior().unwrap_or(0.),
            name: get_name(material.name()),
        }
    }
}
