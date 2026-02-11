use std::ops::{Index, IndexMut};

use super::Vec4;

pub struct Matrix4 {
    pub x: Vec4,
    pub y: Vec4,
    pub z: Vec4,
    pub w: Vec4,
}

impl Matrix4 {
    pub const fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let right_left = right - left;
        let top_bottom = top - bottom;
        let far_near = far - near;

        let x = Vec4::new(2.0 / right_left, 0.0, 0.0, 0.0);
        let y = Vec4::new(0.0, 2.0 / top_bottom, 0.0, 0.0);
        let z = Vec4::new(0.0, 0.0, -2.0 / far_near, 0.0);

        let w = Vec4::new(
            -(right + left) / right_left,
            -(top + bottom) / top_bottom,
            -(far + near) / far_near,
            1.0,
        );

        Self { x, y, z, w }
    }
}

impl Index<usize> for Matrix4 {
    type Output = Vec4;

    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.w,
            _ => panic!("Index out of bounds!"),
        }
    }
}

impl IndexMut<usize> for Matrix4 {
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.w,
            _ => panic!("Index out of bounds!"),
        }
    }
}
