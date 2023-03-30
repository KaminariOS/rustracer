use app::types::*;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UniformBufferObject {

    pub(crate) model_view: Mat4,
    pub(crate) projection: Mat4,
    pub(crate) model_view_inverse: Mat4,
    pub(crate) projection_inverse: Mat4,

    pub(crate) aperture: f32,
    pub(crate) focus_distance: f32,
    pub(crate) heatmap_scale: f32,
    pub(crate) total_number_of_samples: u32,

    pub(crate) number_of_samples: u32,
    pub(crate) number_of_bounces: u32,
    pub(crate) random_seed: u32,
    pub(crate) has_sky: u32,
    pub(crate) show_heatmap: u32,
}