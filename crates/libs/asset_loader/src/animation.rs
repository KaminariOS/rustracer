use crate::geometry::GeoBuilder;
use crate::{get_name, Name, NodeID};
use glam::{Quat, Vec3};
use gltf::animation::util::ReadOutputs;
use gltf::animation::{Channel, Interpolation, Sampler};
use log::warn;

type Float3 = [f32; 3];
type Float4 = [f32; 4];

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
    Translation(Vec<Float3>),
    Rotation(Vec<[f32; 4]>),
    Scale(Vec<Float3>),
    Morph(Vec<Vec<f32>>),
}

impl Property {
    fn len(&self) -> usize {
        match self {
            Self::Translation(a) | Self::Scale(a) => a.len(),
            Self::Rotation(a) => a.len(),
            Self::Morph(a) => a.len(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PropertyOutput {
    Translation(Float3),
    Rotation([f32; 4]),
    Scale(Float3),
    Morph,
}

impl AnimationChannel {
    fn new(channel: Channel<'_>, builder: &GeoBuilder) -> Self {
        let reader = channel.reader(|buffer| Some(&builder.buffers[buffer.index()]));
        let target = channel.target();
        let target_node = target.node().index();
        // let property = target.property().into();
        let input: Vec<_> = reader.read_inputs().unwrap().collect();
        let input_len = input.len();
        let output = reader.read_outputs().unwrap();
        let property = match output {
            ReadOutputs::Translations(t) => Property::Translation(t.collect()),
            ReadOutputs::Rotations(r) => Property::Rotation(r.into_f32().collect()),
            ReadOutputs::Scales(s) => Property::Scale(s.collect()),
            ReadOutputs::MorphTargetWeights(m) => {
                let weights: Vec<_> = m.into_f32().collect();
                let chuck_size = weights.len() / input_len;
                Property::Morph(
                    weights
                        .chunks(chuck_size)
                        .map(|x| x.iter().map(|e| *e).collect())
                        .collect(),
                )
            }
        };
        let sampler = channel.sampler();
        assert_eq!(input_len, property.len());
        Self {
            target: target_node,
            property,
            input,
            interpolation: sampler.interpolation(),
        }
    }

    pub fn get_transform(&self, t: f32) -> PropertyOutput {
        let min = self.input[0];
        let len = self.input.len();
        let max = self.input[len - 1];
        let interval = max - min;
        let t = if t > min {
            (t - min) % interval + min
        } else {
            t
        };
        let mut s = 0;
        let mut e = 0;
        for (i, &d) in self.input[..len - 1].iter().enumerate() {
            if t >= d && t <= self.input[i + 1] {
                s = i;
                e = s + 1;
            }
        }

        let s3 = s * 3;
        let prev_time = self.input[s];
        let next_time = self.input[e];
        let factor = (t - prev_time) / (next_time - prev_time);
        let interpolation = self.interpolation;
        match &self.property {
            Property::Translation(t) => {
                let res = match interpolation {
                    Interpolation::Linear => interpolate_lerp3(t[s], t[e], factor),
                    Interpolation::Step => t[s],
                    Interpolation::CubicSpline => cubic_spline(
                        [t[s3], t[s3 + 1], t[s3 + 2]],
                        prev_time,
                        [t[s3 + 3], t[s3 + 4], t[s3 + 5]],
                        next_time,
                        factor,
                    ),
                };
                PropertyOutput::Translation(res)
            }
            Property::Rotation(r) => {
                let l = Quat::from_array(r[s]);
                let right = Quat::from_array(r[e]);
                let res = match interpolation {
                    Interpolation::Linear | Interpolation::CubicSpline => {
                        l.slerp(right, factor).to_array()
                    }
                    Interpolation::Step => r[s],
                };
                PropertyOutput::Rotation(res)
            }
            Property::Scale(t) => {
                let res = match interpolation {
                    Interpolation::Linear => interpolate_lerp3(t[s], t[e], factor),
                    Interpolation::Step => t[s],
                    Interpolation::CubicSpline => cubic_spline(
                        [t[s3], t[s3 + 1], t[s3 + 2]],
                        prev_time,
                        [t[s3 + 3], t[s3 + 4], t[s3 + 5]],
                        next_time,
                        factor,
                    ),
                };
                PropertyOutput::Scale(res)
            }
            Property::Morph(_) => {
                warn!("Morph unimplemented. Ignore.");
                PropertyOutput::Scale([1.; 3])
            }
        }
    }
}

fn interpolate_lerp3(s: Float3, e: Float3, factor: f32) -> Float3 {
    let start = Vec3::from_array(s);
    let end = Vec3::from_array(e);
    start.lerp(end, factor).to_array()
}

// Stole from Ben
fn cubic_spline(
    source: [Float3; 3],
    source_time: f32,
    target: [Float3; 3],
    target_time: f32,
    amount: f32,
) -> Float3 {
    let source = source.map(|i| Vec3::from_array(i));
    let target = target.map(|i| Vec3::from_array(i));
    let t = amount;
    let p0 = source[1];
    let m0 = (target_time - source_time) * source[2];
    let p1 = target[1];
    let m1 = (target_time - source_time) * target[0];

    let res = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p0
        + (t * t * t - 2.0 * t * t + t) * m0
        + (-2.0 * t * t * t + 3.0 * t * t) * p1
        + (t * t * t - t * t) * m1;
    res.to_array()
}

// https://web.mit.edu/2.998/www/QuaternionReport1.pdf
// Todo: implement squad
// fn cubic_spline4(
//     source: [Float4; 3],
//     source_time: f32,
//     target: [Float4; 3],
//     target_time: f32,
//     amount: f32,
// ) -> Float4 {
//     let source = source.map(|i| Quat::from_array(i));
//     let target = target.map(|i| Quat::from_array(i));
//     let t = amount;
//     let p0 = source[1];
//     let m0 = (target_time - source_time) * source[2];
//     let p1 = target[1];
//     let m1 = (target_time - source_time) * target[0];
//
//     let res = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p0
//         + (t * t * t - 2.0 * t * t + t) * m0
//         + (-2.0 * t * t * t + 3.0 * t * t) * p1
//         + (t * t * t - t * t) * m1;
//     res.to_array()
// }

struct AnimationSampler {}

impl<'a> From<Sampler<'_>> for AnimationSampler {
    fn from(sampler: Sampler<'_>) -> Self {
        sampler.input();
        sampler.output();
        Self {}
    }
}

impl Animation {
    pub fn new(animation: gltf::Animation<'_>, builder: &GeoBuilder) -> Self {
        let index = animation.index();
        let channels = animation
            .channels()
            .map(|c| AnimationChannel::new(c, builder))
            .collect();
        // let sampler= animation.samplers();
        Self {
            index,
            name: get_name!(animation),
            channels,
        }
    }
}
