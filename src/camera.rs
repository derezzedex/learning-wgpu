pub use glam::{Vec3, Mat4};

// #[cfg_attr(rustfmt, rustfmt_skip)]
// const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.0,
//     0.0, 0.0, 0.5, 1.0,
// );


pub struct Camera{
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub near: f32,
    pub far: f32,
    pub velocity: Vec3,
}

impl Camera{
    pub fn get_view(&self) -> Mat4{
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn get_projection(&self) -> Mat4{
        Mat4::perspective_rh_gl(self.fovy, self.aspect, self.near, self.far)
    }
}
