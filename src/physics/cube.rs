use crate::primitives::Vec3;

#[derive(Debug)]
pub struct Cube {
    pub position: Vec3,
    pub size: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub rotation: f32,
    pub angular_velocity: f32,
}

impl Cube {
    pub const fn new(position: Vec3, size: Vec3, velocity: Vec3) -> Self {
        Self { position, size, velocity, mass: 1.0, rotation: 0.0, angular_velocity: 0.0 }
    }

    pub fn apply_force(&mut self, force: Vec3) {
        let acceleration = force / self.mass;
        self.velocity += acceleration;
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;
        self.rotation += self.angular_velocity * delta_time;
    }

    pub fn check_collision(&self, other: &Cube) -> bool {
        let self_min = self.position - self.size / 2.0;
        let self_max = self.position + self.size / 2.0;
        let other_min = other.position - other.size / 2.0;
        let other_max = other.position + other.size / 2.0;

        self_min.x <= other_max.x && self_max.x >= other_min.x &&
        self_min.y <= other_max.y && self_max.y >= other_min.y &&
        self_min.z <= other_max.z && self_max.z >= other_min.z
    }

    pub fn resolve_collision(&mut self, other: &mut Cube) {
        if self.check_collision(other) {
            let normal = (self.position - other.position).normalize();
            let relative_velocity = self.velocity - other.velocity;
            let velocity_along_normal = relative_velocity.dot(normal);

            if velocity_along_normal > 0.0 {
                return;
            }

            let restitution = 0.5; // coefficient of restitution
            let impulse_scalar = -(1.0 + restitution) * velocity_along_normal / (1.0 / self.mass + 1.0 / other.mass);
            let impulse = normal * impulse_scalar;

            self.velocity -= impulse / self.mass;
            other.velocity += impulse / other.mass;
        }
    }
}