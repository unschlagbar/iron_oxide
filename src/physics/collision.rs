use crate::primitives::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum Collision {
    Cube { center: Vec3, size: Vec3 },
    Plane { center: Vec3, size: Vec3 },
}