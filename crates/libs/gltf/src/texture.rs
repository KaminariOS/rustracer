#[derive(Debug, Clone, Copy)]
pub struct Texture {
    pub image_index: usize,
    pub sampler_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Sampler {
    pub mag_filter: MagFilter,
    pub min_filter: MinFilter,
    pub wrap_s: WrapMode,
    pub wrap_t: WrapMode,
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
        Self {
            image_index: texture.source().index(),
            sampler_index: texture.sampler().index().map_or(0, |i| i + 1),
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
