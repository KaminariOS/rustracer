use gltf::animation::{Channel, Sampler};
use crate::{get_name, MeshID, Name};
use crate::geometry::GeoBuilder;

pub struct Animation {
    index: usize,
    name: Name,
}

struct AnimationChannel {

}

impl AnimationChannel {
    fn new(channel: Channel<'_>, builder: &GeoBuilder) -> Self {
        let reader = channel.reader(|buffer| Some(&builder.buffers[buffer.index()]));
        let target = channel.target();
        let target_node = target.node().index();
        let property = target.property();
        let input = reader.read_inputs().unwrap();
        let output = reader.read_outputs().unwrap();
        let sampler = channel.sampler();
        Self {

        }
    }
}

struct AnimationSampler {

}

impl<'a> From<Sampler<'_>> for AnimationSampler {
    fn from(sampler: Sampler<'_>) -> Self {
        sampler.input();
        sampler.output();
        let interpolation = sampler.interpolation();
        Self {

        }
    }
}

impl<'a> From<gltf::Animation<'_>> for Animation {
    fn from(animation: gltf::Animation<'_>) -> Self {
        let index = animation.index();
        let channels = animation.channels();
        let sampler= animation.samplers();
        Self {
            index,
            name: get_name!(animation)
        }
    }
}
