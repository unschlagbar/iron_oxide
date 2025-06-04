use crate::primitives::Vec2;

#[derive(Debug)]
pub struct Rect {
    pub position: Vec2,
    pub size: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
    pub rotation: f32,
    pub angular_velocity: f32,
}

impl Rect {
    pub const fn new(position: Vec2, size: Vec2, velocity: Vec2) -> Self {
        Self { position, size, velocity, mass: 1.0, rotation: 0.0, angular_velocity: 0.0 }
    }

}