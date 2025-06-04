use crate::physics::ImplRigitBody;



#[derive(Debug)]
#[allow(unused)]
pub struct System {
    gravity: f32,
}

impl System {
    pub fn new() -> Self {
        Self { gravity: -9.81 }
    }

    #[allow(unused)]
    pub fn update(&self, objects: &mut [impl ImplRigitBody], delta_time: f32) {

       todo!()
    }
}