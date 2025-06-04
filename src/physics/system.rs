use crate::physics::{Collision, ImplRigitBody};

#[derive(Debug)]
#[allow(unused)]
pub struct System {
    gravity: f32,
}

impl System {
    pub fn new() -> Self {
        Self { gravity: -9.81 }
    }

    pub fn update(&self, objects: &mut [impl ImplRigitBody], delta_time: f32) {

        for object in 0..objects.len() {
            objects[object].rigit_body().update(delta_time);

            for other in 0..objects.len() {
                if object != other {
                    let other_object = unsafe { &mut *(&mut objects[other] as *mut dyn ImplRigitBody) };
                    let object = &mut objects[object];
                    let collision = object.collision();
                    let other_collision = other_object.collision();

                    match (collision, other_collision) {
                        (Collision::Cube { center: center1, size: size1 }, Collision::Cube { center: center2, size: size2 }) => {
                            let distance = (center1 - center2).magnitude();
                            let min_distance = (size1.y + size2.y) / 2.0;

                            if distance < min_distance {
                                let direction = {
                                    if distance == 0.0 {
                                        center1 - center2
                                    } else {
                                        (center1 - center2) / distance
                                    }
                                };
                                let overlap = min_distance - distance;

                                let rigitbody = object.rigit_body();
                                let other_rigitbody = other_object.rigit_body();

                                rigitbody.position += rigitbody.position_lock * direction * overlap;


                                let relative_velocity =  rigitbody.velocity - other_rigitbody.velocity;
                                let velocity_along_normal = relative_velocity.dot(direction);

                                if velocity_along_normal > 0.0 {
                                    continue;
                                }

                                let speed = direction * relative_velocity;
                                let impulse = (speed * 2.0) / (rigitbody.mass + other_rigitbody.mass);

                                rigitbody.velocity -= impulse * other_rigitbody.mass * direction;
                                other_rigitbody.velocity += impulse * rigitbody.mass * direction;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}