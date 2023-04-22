use std::default::Default;
use glam::Mat4;
use gltf::buffer;
use log::warn;

use crate::scene_graph::Node;
use crate::{get_index, get_index_array, get_name, Name, NodeID};

pub const MAX_JOINTS: usize = 256;
pub type SkinRaw = [JointRaw; MAX_JOINTS];

pub struct Skin {
    pub index: usize,
    name: Name,
    joints: Vec<Joint>,
}

impl Skin {
    pub fn new(skin: gltf::Skin, data: &[buffer::Data]) -> Self {
        // let reader = skin.inverse_bind_matrices();
        let joints: Vec<_> = get_index_array!(skin.joints());
        skin.skeleton();
        let ibms: Vec<_> = skin
            .reader(|b| Some(&data[b.index()]))
            .read_inverse_bind_matrices()
            .unwrap()
            .map(|m| Mat4::from_cols_array_2d(&m))
            .collect();
        assert_eq!(ibms.len(), joints.len());
        let joints = joints.into_iter().zip(ibms).map(Joint::from).collect();
        // let reader = skin.reader();
        Self {
            index: skin.index(),
            name: get_name!(skin),
            joints,
        }
    }

    pub fn get_skin_matrices(&self, nodes: &[Node]) -> SkinRaw {
        let len = self.joints.len();
        if len > MAX_JOINTS {
            warn!("Too many joints: {}; current max: {}", len, MAX_JOINTS);
        }
        let mut res: SkinRaw = [Default::default(); MAX_JOINTS];
        let len = len.min(MAX_JOINTS);
        self.joints[0..len].iter()
            .enumerate()
            .for_each(|(i, j)| {
            res[i] = (j.compute_skinning_matrix(nodes)).into();
        });
        res
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct JointRaw {
    matrix: [f32; 16],
}

impl From<Mat4> for JointRaw {
    fn from(value: Mat4) -> Self {
        Self {
            matrix: value.to_cols_array()
        }
    }
}

pub struct Joint {
    node: NodeID,
    ibm: Mat4,
}

impl From<(usize, Mat4)> for Joint {
    fn from((node, ibm): (usize, Mat4)) -> Self {
        Self { node, ibm }
    }
}

impl Joint {
    pub fn compute_skinning_matrix(&self, nodes: &[Node]) -> Mat4 {
        nodes[self.node].get_world_transform() * self.ibm
    }
}
