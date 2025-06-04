use crate::primitives::Vec3;

use super::Collision;



pub trait ImplRigitBody {
    fn velocity(&mut self) -> &mut Vec3;
    fn position(&mut self) -> &mut Vec3;
    fn rigit_body(&mut self) -> &mut RigitBody;
    fn collision(&mut self) -> Collision;
}

#[derive(Debug, Clone)]
pub struct RigitBody {
    pub gravity: bool,
    pub on_ground: bool,
    pub position: Vec3,
    pub size: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub position_lock: Vec3,
}

impl RigitBody {
    pub fn update(&mut self, delta_time: f32) {
        if self.gravity {
            self.velocity.y += -9.81 * delta_time;
        }
        self.position += self.velocity * delta_time * self.position_lock;
    }
}