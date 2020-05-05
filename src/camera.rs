pub use cgmath::{Point3, Vector3, Matrix4, Deg};

// #[cfg_attr(rustfmt, rustfmt_skip)]
// const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.0,
//     0.0, 0.0, 0.5, 1.0,
// );


pub struct Camera{
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub near: f32,
    pub far: f32,
    pub velocity: Vector3<f32>,
}

impl Camera{
    pub fn get_view(&self) -> Matrix4<f32>{
        cgmath::Matrix4::look_at(self.eye, self.target, self.up)
    }

    pub fn get_projection(&self) -> Matrix4<f32>{
        cgmath::perspective(Deg(self.fovy), self.aspect, self.near, self.far)
    }
}
