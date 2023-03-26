use nalgebra::{Matrix4, Point3, UnitQuaternion, UnitVector3, Vector3};

pub type Point = Point3<f32>;
pub type Vec3 = Vector3<f32>;
pub type UniVec3 = UnitVector3<f32>;
// pub type Vec4 = Vector4<f32>;
// pub type Mat3 = Matrix3<f32>;
pub type Mat4 = Matrix4<f32>;
pub type Quat = UnitQuaternion<f32>;

pub fn a3toa4<T: Copy>(a3: &[T], w: T) -> [T; 4] {
    [a3[0], a3[1], a3[2], w]
}
