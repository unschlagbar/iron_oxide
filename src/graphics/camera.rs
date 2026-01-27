use cgmath::{Matrix4, Point3, Vector3};

use crate::primitives::{Vec2, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub moved: bool,
}

impl Camera {
    pub fn process_mouse_movement(&mut self, delta: Vec2, sensitivity: f32) {
        self.yaw += delta.x * sensitivity;
        self.pitch += delta.y * sensitivity;
        self.pitch = self.pitch.clamp(-89.0, 89.0); // Begrenze Pitch
        self.moved = true;
    }

    pub fn process_movement(&mut self, delta: Vec3, speed: f32) {
        let front = Vec3::new(
            self.yaw.to_radians().cos(),
            0.0,
            self.yaw.to_radians().sin(),
        )
        .normalize();
        let right = front.cross(Vec3::new(0.0, -1.0, 0.0)).normalize();

        self.position += front * delta.z * speed;
        self.position += right * delta.x * speed;
        self.position.y += delta.y * speed;
        self.moved = true;
    }

    pub fn view(&mut self) -> Matrix4<f32> {
        self.moved = false;
        let yaw_radians = self.yaw.to_radians();
        let pitch_radians = self.pitch.to_radians();

        let front = Vec3::new(
            yaw_radians.cos() * pitch_radians.cos(),
            pitch_radians.sin(),
            yaw_radians.sin() * pitch_radians.cos(),
        )
        .normalize();

        Matrix4::look_to_rh(
            Point3::new(self.position.x, self.position.y, self.position.z),
            front.into(),
            Vector3::new(0.0, -1.0, 0.0),
        )
    }

    pub fn projection(&self, aspect_ratio: f32) -> Matrix4<f32> {
        cgmath::perspective(cgmath::Deg(self.fov), aspect_ratio, self.near, self.far)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec3::new(0.0, 0.0, -10.0),
            yaw: 90.0,
            pitch: 0.0,
            fov: 45.0,
            near: 0.1,
            far: 1000.0,
            moved: true,
        }
    }
}
