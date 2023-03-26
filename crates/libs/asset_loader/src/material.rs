use crate::{to_owned_string, Name};
use gltf::material::{AlphaMode, NormalTexture, OcclusionTexture};
use gltf::texture;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TextureInfo {
    pub texture_index: i32,
    // Most glTF only uses tex_coord 0
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

    fn is_none(&self) -> bool {
        self.texture_index == -1
    }
}

pub struct Material {
    pub(crate) name: Name,
    pub(crate) index: usize,

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
        let mut metallic_roughness_texture = TextureInfo::new(pbr.metallic_roughness_texture());
        if metallic_roughness_texture.is_none() {
            metallic_roughness_texture = TextureInfo::new(
                material
                    .pbr_specular_glossiness()
                    .and_then(|sg| sg.diffuse_texture()),
            )
        }

        Self {
            alpha_mode: material.alpha_mode(),
            alpha_cutoff: material.alpha_cutoff(),
            double_sided: material.double_sided(),

            base_color: pbr.base_color_factor(),
            base_color_texture: TextureInfo::new(pbr.base_color_texture()),

            metallic_factor: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            metallic_roughness_texture,

            normal_texture: TextureInfo::new_normal(material.normal_texture()),

            emissive_factor: [em[0], em[1], em[2], 0.],
            emissive_texture: TextureInfo::new(material.emissive_texture()),

            occlusion_texture: TextureInfo::new_occ(material.occlusion_texture()),
            ior: material.ior().unwrap_or(0.),
            name: material.name().map(to_owned_string),
            index: material.index().unwrap_or(0),
        }
    }
}
