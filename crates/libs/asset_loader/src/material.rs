use std::collections::{HashSet};
use crate::{Name, a3toa4, get_name};
use gltf::material::{AlphaMode, NormalTexture, OcclusionTexture, PbrMetallicRoughness, PbrSpecularGlossiness, Specular, Transmission, Volume};
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

#[derive(Clone)]
pub enum MaterialType {
    MetallicRoughness,
}

impl TextureInfo {
    fn new(info: Option<texture::Info>) -> Self {
        info.map(|t| Self {
            texture_index: (1 + t.texture().index()) as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }

    fn new_normal(info: Option<NormalTexture>) -> Self {
        info.map(|t| Self {
            texture_index: (1 + t.texture().index()) as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }

    fn new_occ(info: Option<OcclusionTexture>) -> Self {
        info.map(|t| Self {
            texture_index: (1 + t.texture().index()) as _,
            tex_coord: t.tex_coord() as _,
        })
        .unwrap_or_default()
    }

    fn is_none(&self) -> bool {
        self.texture_index == -1
    }

    fn is_some(&self) -> bool {
        !self.is_none()
    }
}

#[derive(Clone)]
pub struct Material {
    pub(crate) name: Name,
    pub(crate) index: usize,

    pub unlit: bool,

    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: bool,

    pub base_color: [f32; 4],
    pub base_color_texture: TextureInfo,

    pub metallic_roughness_info: MetallicRoughnessInfo,

    pub normal_texture: TextureInfo,

    pub emissive_factor: [f32; 4],
    pub emissive_texture: TextureInfo,

    pub occlusion_texture: TextureInfo,
    pub ior: f32,
    pub material_type: MaterialType,
    transmission: Option<TransmissionInfo>,
    volume_info: Option<VolumeInfo>,
    specular_info: Option<SpecularInfo>,
}

impl Material {
    pub fn has_normal_texture(&self) -> bool {
        !self.normal_texture.is_none()
    }
}

pub fn find_linear_textures(materials: &[Material]) -> HashSet<usize> {
    // https://gltf-transform.donmccurdy.com/classes/core.material.html
    // Textures containing color data (baseColorTexture, emissiveTexture) are sRGB.
    // All other textures are linear. Like other resources, textures should be reused when possible.
    let mut set: HashSet<_> = HashSet::new();
        materials.iter().for_each(|m|{
        if m.normal_texture.is_some() {
            set.insert(m.normal_texture.texture_index as usize);
        }
        if m.metallic_roughness_info.metallic_roughness_texture.is_some() {
            set.insert(m.metallic_roughness_info.metallic_roughness_texture.texture_index as usize);
        }
        if let Some(tr) = m.transmission
            .filter(|t| t.transmission_texture.is_some())
            .map(|t| t.transmission_texture.texture_index)
        {
            set.insert(tr as _);
        }
        if let Some(sp) = m.specular_info
            .filter(|s| s.specular_texture.is_some())
            .map(|s| s.specular_texture.texture_index) {
            set.insert(sp as _);
        }
    });
    set
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MaterialRaw {
    pub alpha_mode: u32,
    pub alpha_cutoff: f32,
    pub _padding: [f32; 3],
    pub double_sided: u32,

    pub base_color_texture: TextureInfo,
    // 4 int
    pub base_color: [f32; 4],
    // 4 int
    pub metallic_roughness_info: MetallicRoughnessInfo,

    // 4 int
    pub normal_texture: TextureInfo,

    pub emissive_texture: TextureInfo,
    // 4 int
    pub emissive_factor: [f32; 4],
    // 4 int
    pub occlusion_texture: TextureInfo,
    pub ior: f32,
    pub unlit: u32,
    // 4 int
    pub transmission: TransmissionInfo,
    pub volume_info: VolumeInfo,
    specular_info: SpecularInfo,
}

impl From<&Material> for MaterialRaw {
    fn from(value: &Material) -> Self {
        Self {
            alpha_mode: value.alpha_mode as _,
            alpha_cutoff: value.alpha_cutoff.unwrap_or_default(),
            _padding: [0.; 3],
            double_sided: value.double_sided.into(),
            base_color_texture: value.base_color_texture,
            base_color: value.base_color,
            metallic_roughness_info: value.metallic_roughness_info,
            normal_texture: value.normal_texture,
            emissive_texture: value.emissive_texture,
            emissive_factor: value.emissive_factor,
            occlusion_texture: value.occlusion_texture,
            ior: value.ior,
            unlit: value.unlit.into(),
            transmission: value.transmission.unwrap_or_default(),
            volume_info: value.volume_info.unwrap_or_default(),
            specular_info: value.specular_info.unwrap_or_default(),
        }
    }
}

// 8 floats
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VolumeInfo {
    attenuation_color: [f32; 3],
    thickness_factor: f32,
    thickness_texture: TextureInfo,
    attenuation_distance: f32,
    exists: u32,
}

impl Default for VolumeInfo {
    fn default() -> Self {
        Self {
            attenuation_color: [1.; 3],
            thickness_factor: 0.0,
            thickness_texture: Default::default(),
            attenuation_distance: f32::MAX,
            exists: false.into(),
        }
    }
}

impl From<Volume<'_>> for VolumeInfo {
    fn from(volume: Volume) -> Self {
        volume.attenuation_color();
        volume.thickness_factor();
        volume.attenuation_distance();
        volume.thickness_texture();
        Self {
            attenuation_color: volume.attenuation_color(),
            thickness_factor: volume.thickness_factor(),
            thickness_texture: TextureInfo::new(volume.thickness_texture()),
            attenuation_distance: volume.attenuation_distance(),
            exists: 1,
        }
    }
}

// 4 floats
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MetallicRoughnessInfo {
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub metallic_roughness_texture: TextureInfo,
}

impl From<PbrMetallicRoughness<'_>> for MetallicRoughnessInfo {
    fn from(phr: PbrMetallicRoughness) -> Self {
        Self {
            metallic_factor: phr.metallic_factor(),
            roughness_factor: phr.roughness_factor(),
            metallic_roughness_texture: TextureInfo::new(phr.metallic_roughness_texture()),
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct TransmissionInfo {
    transmission_texture: TextureInfo,
    transmission_factor: f32,
    exist: u32
}

impl From<Transmission<'_>> for TransmissionInfo {
    fn from(transmission: Transmission) -> Self {
        Self {
            transmission_texture: TextureInfo::new(transmission.transmission_texture()),
            transmission_factor: transmission.transmission_factor(),
            exist: 1,
        }
    }
}
struct SpecularGlossiness {
    diffuse_factor: [f32; 4],
    diffuse_texture: TextureInfo,
    specular_glossiness_texture: TextureInfo,
    glossiness_factor: f32,
    specular_factor: [f32; 3],
}

impl<'a> From<PbrSpecularGlossiness<'_>> for SpecularGlossiness {
    fn from(pbr: PbrSpecularGlossiness<'_>) -> Self {
        Self {
            diffuse_factor: pbr.diffuse_factor(),
            glossiness_factor: pbr.glossiness_factor(),
            specular_factor: pbr.specular_factor(),
            diffuse_texture: TextureInfo::new(pbr.diffuse_texture()),
            specular_glossiness_texture: TextureInfo::new(pbr.specular_glossiness_texture())
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SpecularInfo {
    specular_texture: TextureInfo,
    specular_color_texture: TextureInfo,
    specular_color_factor: [f32; 4],
    specular_factor: f32,
    exist: u32,
    _padding: f64,
}

impl<'a> From<Specular<'_>> for SpecularInfo {
    fn from(sp: Specular) -> Self {
        Self {
            specular_texture: TextureInfo::new(sp.specular_texture()),
            specular_color_texture: TextureInfo::new(sp.specular_color_texture()),
            specular_color_factor: a3toa4(&sp.specular_color_factor(), 0.),
            specular_factor: sp.specular_factor(),
            exist: true.into(),
            _padding: 0.0,
        }
    }
}

impl Default for SpecularInfo {
    fn default() -> Self {
        Self {
            specular_texture: Default::default(),
            specular_color_texture: Default::default(),
            specular_color_factor: [1.; 4],
            specular_factor: 1.0,
            exist: false.into(),
            _padding: 0.0,
        }
    }
}

impl<'a> From<gltf::Material<'_>> for Material {
    fn from(material: gltf::Material) -> Self {
        let pbr = material.pbr_metallic_roughness();
        let specular = material.specular().map(SpecularInfo::from);
        let em = material.emissive_factor();
        let unlit = material.unlit();
        let _sg = material.pbr_specular_glossiness().map(SpecularGlossiness::from);

        let mut base_color_texture = TextureInfo::new(pbr.base_color_texture());
        if base_color_texture.is_none() {
            base_color_texture = TextureInfo::new(
                material
                    .pbr_specular_glossiness()
                    .and_then(|sg| sg.diffuse_texture()),
            )
        }
        let volume_info = material.volume().map(VolumeInfo::from);

        Self {
            alpha_mode: material.alpha_mode(),
            alpha_cutoff: material.alpha_cutoff(),
            double_sided: material.double_sided(),

            base_color: pbr.base_color_factor(),
            base_color_texture,
            normal_texture: TextureInfo::new_normal(material.normal_texture()),
            metallic_roughness_info: pbr.into(),
            emissive_factor: a3toa4(&em, 0.),
            emissive_texture: TextureInfo::new(material.emissive_texture()),

            occlusion_texture: TextureInfo::new_occ(material.occlusion_texture()),
            // glTF default
            ior: material.ior().unwrap_or(1.5),
            name: get_name!(material),
            index: material.index().unwrap_or(0),
            material_type: MaterialType::MetallicRoughness,
            transmission: material.transmission().map(TransmissionInfo::from),
            volume_info,
            unlit,
            specular_info: specular,
        }
    }
}
