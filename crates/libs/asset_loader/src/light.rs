use glam::{Mat4, Vec4, Vec4Swizzles};
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
            color: Vec4::from_array(a3toa4(&self.color, 0.)),
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
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LightRaw {
    pub color: Vec4,
    pub(crate) transform: Vec4,
    pub kind: u32,
    pub range: f32,
    pub intensity: f32,
    pub _padding: u32
}

impl LightRaw {
    pub fn is_dir(&self) -> bool {
        self.kind == LightType::DIRECTIONAL as u32
    }

    // pub fn get_angles(&self) -> [f32; 2] {
    //
    //     const PI: f32 = std::f32::consts::PI;
    //     let dir = self.transform.xyz().normalize();
    //     let theta = self.transform.y.acos();
    //     let phi = if self.transform.x != 0.{
    //         (self.transform.y / self.transform.x).atan()
    //     } else {
    //         if self.transform.y >= 0. {
    //             PI * 0.5
    //         } else {
    //             - PI * 0.5
    //         }
    //     };
    //     [theta, phi]
    // }

    pub fn update_angles(&mut self, [theta, phi]: [f32; 2]) {
        self.transform[0] = -theta.sin() * phi.sin();
        self.transform[1] = -theta.cos();
        self.transform[2] = -theta.sin() * phi.cos();
    }

    pub fn update_color(&mut self, color: [f32; 4]) {
        self.color = Vec4::from_array(color);
    }
}

impl Default for LightRaw {
    fn default() -> Self {
        Self {
            color: Vec4::from_array([1.; 4]),
            transform: Vec4::ONE,
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
