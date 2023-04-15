use glam::Mat4;
use gltf::buffer;
use vulkan::Fence;
use crate::{get_index, get_index_array, get_name, Name, NodeID};
use crate::scene_graph::Node;

pub struct Skin {
    index: usize,
    name: Name,
    joints: Vec<Joint>,
}

impl Skin {
    pub fn new(skin: gltf::Skin, data: &[buffer::Data]) -> Self {
        // let reader = skin.inverse_bind_matrices();
        let joints: Vec<_> = get_index_array!(skin.joints());
        skin.skeleton();
        let ibms: Vec<_> = skin.reader(|b| Some(&data[b.index()]))
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
}

pub struct Joint {
    node: NodeID,
    ibm: Mat4,
}

impl From<(usize, Mat4)> for Joint {
    fn from((node, ibm): (usize, Mat4)) -> Self {
        Self {
            node,
            ibm,
        }
    }
}

impl Joint {
    pub fn compute_skinning_matrix(&self, global_transform: Mat4, nodes: &[Node]) -> Mat4 {
        global_transform.inverse() * nodes[self.node].get_world_transform() * self.ibm
    }
}