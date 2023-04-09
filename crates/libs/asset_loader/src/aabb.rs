use std::ops::Mul;
use glam::{Mat4, Vec3, Vec4};
use gltf::mesh::Bounds;

/// Axis aligned bounding box.
#[derive(Copy, Clone, Debug)]
pub struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    /// Create a new AABB.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }
}

impl Aabb{
    /// Compute the union of several AABBs.
    pub fn union(aabbs: &[Aabb]) -> Option<Self> {
        if aabbs.is_empty() {
            None
        } else if aabbs.len() == 1 {
            Some(aabbs[0])
        } else {
            // let partial_cmp = |a, b| a.partial_cmp(b);
            let by_key = |a: &f32, b: &f32| a.partial_cmp(b).unwrap();
            let min_x = aabbs.iter().map(|aabb| aabb.min.x)
                .min_by(by_key).unwrap();
            let min_y = aabbs.iter().map(|aabb| aabb.min.y)
                .min_by(by_key).unwrap();
            let min_z = aabbs.iter().map(|aabb| aabb.min.z)
                .min_by(by_key).unwrap();
            let min = Vec3::new(min_x, min_y, min_z);

            let max_x = aabbs.iter().map(|aabb| aabb.max.x).max_by(by_key).unwrap();
            let max_y = aabbs.iter().map(|aabb| aabb.max.y).max_by(by_key).unwrap();
            let max_z = aabbs.iter().map(|aabb| aabb.max.z).max_by(by_key).unwrap();
            let max = Vec3::new(max_x, max_y, max_z);

            Some(Aabb::new(min, max))
        }
    }

    /// Get the size of the larger side of the AABB.
    pub fn get_larger_side_size(&self) -> f32 {
        let size = self.max - self.min;
        let x = size.x.abs();
        let y = size.y.abs();
        let z = size.z.abs();

        if x > y && x > z {
            x
        } else if y > z {
            y
        } else {
            z
        }
    }

    /// Get the center of the AABB.
    pub fn get_center(&self) -> Vec3 {
        let two = 2.;
        self.min + (self.max - self.min) / two
    }
}

/// Transform the AABB by multiplying it with a Matrix4.
impl Mul<Mat4> for Aabb {
    type Output = Aabb;

    fn mul(self, rhs: Mat4) -> Self::Output {
        let min = self.min;
        let min = rhs * Vec4::new(min.x, min.y, min.z, 1.);

        let max = self.max;
        let max = rhs * Vec4::new(max.x, max.y, max.z, 1.);

        Aabb::new(min.truncate(), max.truncate())
    }
}

/// Scale the AABB by multiplying it by a BaseFloat
impl Mul<f32> for Aabb {
    type Output = Aabb;

    fn mul(self, rhs: f32) -> Self::Output {
        Aabb::new(self.min * rhs, self.max * rhs)
    }
}


pub fn get_aabb(bounds: &Bounds<[f32; 3]>) -> Aabb {
    let min = bounds.min;
    let min = Vec3::from_array(min);

    let max = bounds.max;
    let max = Vec3::from_array(max);

    Aabb::new(min, max)
}
