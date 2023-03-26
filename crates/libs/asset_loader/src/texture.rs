use crate::{to_owned_string, Name};

#[derive(Debug, Clone)]
pub struct Texture {
    pub image_index: usize,
    pub texture_index: usize,
    pub sampler_index: usize,
    pub name: Name,
}

#[derive(Debug, Clone, Copy)]
pub struct Sampler {
    pub mag_filter: MagFilter,
    pub min_filter: MinFilter,
    pub wrap_s: WrapMode,
    pub wrap_t: WrapMode,
    pub(crate) index: usize,
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler {
            mag_filter: MagFilter::Linear,
            min_filter: MinFilter::LinearMipmapLinear,
            wrap_s: WrapMode::Repeat,
            wrap_t: WrapMode::Repeat,
            index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MagFilter {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy)]
pub enum MinFilter {
    Nearest,
    Linear,
    NearestMipmapNearest,
    LinearMipmapNearest,
    NearestMipmapLinear,
    LinearMipmapLinear,
}

#[derive(Debug, Clone, Copy)]
pub enum WrapMode {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

impl<'a> From<gltf::Texture<'a>> for Texture {
    fn from(texture: gltf::Texture) -> Self {
        // println!("Tex: {} Image:{} sampler: {}", texture.index(), texture.source().index(),
        //          texture.sampler().index().map_or(0, |i| i + 1)
        // );
        Self {
            image_index: texture.source().index(),
            texture_index: texture.index(),
            sampler_index: texture.sampler().index().map_or(0, |i| i + 1),
            name: texture.name().map(to_owned_string),
        }
    }
}

impl<'a> From<gltf::texture::Sampler<'a>> for Sampler {
    fn from(sampler: gltf::texture::Sampler<'a>) -> Self {
        Self {
            mag_filter: sampler
                .mag_filter()
                .unwrap_or(gltf::texture::MagFilter::Linear)
                .into(),
            min_filter: sampler
                .min_filter()
                .unwrap_or(gltf::texture::MinFilter::Linear)
                .into(),
            wrap_s: sampler.wrap_s().into(),
            wrap_t: sampler.wrap_t().into(),
            index: sampler.index().map_or(0, |i| i + 1),
        }
    }
}

impl From<gltf::texture::MagFilter> for MagFilter {
    fn from(wrapping_mode: gltf::texture::MagFilter) -> Self {
        match wrapping_mode {
            gltf::texture::MagFilter::Linear => Self::Linear,
            gltf::texture::MagFilter::Nearest => Self::Nearest,
        }
    }
}

impl From<gltf::texture::MinFilter> for MinFilter {
    fn from(wrapping_mode: gltf::texture::MinFilter) -> Self {
        match wrapping_mode {
            gltf::texture::MinFilter::Linear => Self::Linear,
            gltf::texture::MinFilter::Nearest => Self::Nearest,
            gltf::texture::MinFilter::LinearMipmapLinear => Self::LinearMipmapLinear,
            gltf::texture::MinFilter::LinearMipmapNearest => Self::LinearMipmapNearest,
            gltf::texture::MinFilter::NearestMipmapLinear => Self::NearestMipmapLinear,
            gltf::texture::MinFilter::NearestMipmapNearest => Self::NearestMipmapNearest,
        }
    }
}

impl From<gltf::texture::WrappingMode> for WrapMode {
    fn from(wrapping_mode: gltf::texture::WrappingMode) -> Self {
        match wrapping_mode {
            gltf::texture::WrappingMode::ClampToEdge => Self::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => Self::MirroredRepeat,
            gltf::texture::WrappingMode::Repeat => Self::Repeat,
        }
    }
}
