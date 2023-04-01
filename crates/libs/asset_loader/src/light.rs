use glam::{Mat4, Vec4};
use gltf::khr_lights_punctual::Kind;
use log::error;
use crate::{a3toa4, Name, to_owned_string};

pub struct Light {
    pub index: usize,
    color: [f32; 3],
    name: Name,
    kind: Kind,
    range: f32,
}

impl Light {
    pub fn to_raw(&self, transform: Mat4) -> LightRaw {
        let kind = LightType::from(&self.kind);
        LightRaw {
            color: a3toa4(&self.color, 0.),
            transform: kind.get_transform(transform),
            kind:  kind as _,
            _padding: [0; 3],
        }
    }
}

#[derive(Copy, Clone)]
enum LightType {
    DIRECTIONAL = 0,
    POINT = 1,
    SPOT = 2
}

impl LightType {
    fn get_transform(&self, transform: Mat4) -> Vec4 {
        transform * match self {
            LightType::DIRECTIONAL => {
                Vec4::from_array([0., 0., 0.1, 0.])
            }
            LightType::POINT | LightType::SPOT => {
                Vec4::from_array([0.; 4])
            }
        }
    }
}

impl From<&Kind> for LightType {
    fn from(value: &Kind) -> Self {
        match value {
            Kind::Directional => Self::DIRECTIONAL,
            Kind::Point => Self::POINT,
            Kind::Spot { inner_cone_angle: _, outer_cone_angle: _ } => {
                error!("Unimplemented; treat spot light as point light");
                Self::POINT
            }
        }
    }


}

#[repr(C)]
pub struct LightRaw {
    color: [f32; 4],
    transform: Vec4,
    kind: u32,
    _padding: [u32; 3]
}

impl<'a> From<gltf::khr_lights_punctual::Light<'a>> for Light {
    fn from(light: gltf::khr_lights_punctual::Light) -> Self {
        Self {
            index: light.index(),
            color: light.color(),
            name: light.name().map(to_owned_string),
            kind: light.kind(),
            range: light.range().unwrap_or(f32::MAX),
        }
    }
}
