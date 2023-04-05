use glam::{Mat4, Quat, Vec3};
use gltf::animation::{Channel, Interpolation, Sampler};
use gltf::animation::util::ReadOutputs;
use crate::{get_name, MeshID, Name, NodeID};
use crate::geometry::GeoBuilder;

pub struct Animation {
    pub index: usize,
    name: Name,
    pub channels: Vec<AnimationChannel>,
}

pub struct AnimationChannel {
    pub target: NodeID,
    property: Property,
    input: Vec<f32>,
    interpolation: Interpolation,
}

enum Property {
    Translation(Vec<[f32; 3]>),
    Rotation(Vec<[f32; 4]>),
    Scale(Vec<[f32; 3]>),
}

impl Property {
    fn len(&self) -> usize {
        match self
        {
            Self::Translation(a) | Self::Scale(a) => a.len(),
            Self::Rotation(a) => a.len(),
        }
    }
}

// impl From<gltf::animation::Property> for Property {
//     fn from(prop: gltf::animation::Property) -> Self {
//         match prop {
//             gltf::animation::Property::Translation => Self::Transition,
//             gltf::animation::Property::Rotation => Self::Rotation,
//             gltf::animation::Property::Scale => Self::Scale,
//             _ => unimplemented!()
//         }
//     }
// }

#[derive(Clone, Copy)]
pub enum PropertyOutput {
    Translation([f32; 3]),
    Rotation([f32; 4]),
    Scale([f32; 3]),
}
impl AnimationChannel {
    fn new(channel: Channel<'_>, builder: &GeoBuilder) -> Self {
        let reader = channel.reader(|buffer| Some(&builder.buffers[buffer.index()]));
        let target = channel.target();
        let target_node = target.node().index();
        // let property = target.property().into();
        let input: Vec<_> = reader.read_inputs().unwrap().collect();
        let output = reader.read_outputs().unwrap();
        let property = match output {
            ReadOutputs::Translations(t) => {Property::Translation(t.collect())}
            ReadOutputs::Rotations(r) => {Property::Rotation(r.into_f32().collect())}
            ReadOutputs::Scales(s) => {Property::Scale(s.collect())}
            ReadOutputs::MorphTargetWeights(_) => unimplemented!()
        };
        let sampler = channel.sampler();
        assert_eq!(input.len(), property.len());
        Self {
            target: target_node,
            property,
            input,
            interpolation: sampler.interpolation(),
        }
    }

    pub fn get_transform(&self, mut t: f32) -> PropertyOutput {
        let min = self.input[0];
        let len = self.input.len();
        let max = self.input[len - 1];
        let interval = max - min;
        t = t.max(min);
        let t = (t - min) % interval + min;
        let mut s = 0;
        let mut e = 0;
        for (i, &d) in self.input[..len - 1].iter().enumerate() {
            if t >= d && t <= self.input[i + 1] {
                s = i;
                e = s + 1;
            }
        }
        let factor = (t - self.input[s]) / (self.input[e] - self.input[s]);
        //     interpolation::cub_bez()
        use interpolation::lerp;
        match &self.property  {
            Property::Translation(t) => {
                let translation: Vec<_> = t[s].iter().zip(t[e].iter())
                    .map(|(l, r)| lerp(l, r, &factor)).collect();
                PropertyOutput::Translation([translation[0], translation[1], translation[2]])
                // Mat4::from_translation(Vec3::from_slice(translation.as_slice()))
            }
            Property::Rotation(r) => {
                let l = Quat::from_array(r[s]);
                let r = Quat::from_array(r[e]);
                PropertyOutput::Rotation(l.slerp(r, factor).to_array())
            }
            Property::Scale(sv) => {
                let scale:Vec<_> = sv[s].iter()
                    .zip(sv[e].iter())
                    .map(|(l, r)| lerp(l, r, &factor)).collect();
                PropertyOutput::Scale([scale[0], scale[1], scale[2]])
                // Mat4::from_scale(Vec3::from_slice(scale.as_slice()))
            }
        }
    }
}

struct AnimationSampler {

}

impl<'a> From<Sampler<'_>> for AnimationSampler {
    fn from(sampler: Sampler<'_>) -> Self {
        sampler.input();
        sampler.output();
        Self {

        }
    }
}

impl Animation {
    pub fn new(animation: gltf::Animation<'_>, builder: &GeoBuilder) -> Self {
        let index = animation.index();
        let channels = animation.channels().map(|c| AnimationChannel::new(c, builder)).collect();
        // let sampler= animation.samplers();
        Self {
            index,
            name: get_name!(animation),
            channels,
        }
    }
}
