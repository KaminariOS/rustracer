use glam::{Mat4, Vec4};
use gltf::khr_lights_punctual::Kind;
use log::{error, info};
use crate::{a3toa4, get_name, Name};

pub struct Light {
    pub index: usize,
    color: [f32; 3],
    name: Name,
    kind: LightType,
    range: f32,
    intensity: f32,
}

impl Light {
    pub fn to_raw(&self, transform: Mat4) -> LightRaw {
        let kind = self.kind;
        LightRaw {
            color: a3toa4(&self.color, 0.),
            transform: kind.get_transform(transform),
            kind:  kind as _,
            range: self.range,
            intensity: self.intensity,
            _padding: 0,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
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

impl From<Kind> for LightType {
    fn from(value: Kind) -> Self {
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
#[derive(Copy, Clone)]
pub struct LightRaw {
    color: [f32; 4],
    transform: Vec4,
    kind: u32,
    range: f32,
    intensity: f32,
    _padding: u32
}

impl LightRaw {
    pub fn is_dir(&self) -> bool {
        self.kind == LightType::DIRECTIONAL as u32
    }
}

impl Default for LightRaw {
    fn default() -> Self {
        Self {
            color: [0.; 4],
            transform: Default::default(),
            kind: LightType::POINT as _,
            range: 0.0,
            intensity: 0.,
            _padding: 0,
        }
    }
}

impl<'a> From<gltf::khr_lights_punctual::Light<'a>> for Light {
    fn from(light: gltf::khr_lights_punctual::Light) -> Self {
        Self {
            index: light.index(),
            color: light.color(),
            name: get_name!(light),
            kind: light.kind().into(),
            range: light.range().unwrap_or(f32::MAX),
            intensity: light.intensity(),
        }
    }
}

pub fn report_lights(lights: &[Light]) {
    let dirs = lights.iter().filter(|l| l.kind == LightType::DIRECTIONAL).count();
    let points = lights.iter().filter(|l| l.kind == LightType::POINT).count();
    info!("Directional lights: {}; point lights: {}", dirs, points);
}
