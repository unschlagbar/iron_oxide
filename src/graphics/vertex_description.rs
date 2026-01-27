use std::ptr;

use ash::vk;

pub trait VertexDescription: Sized + Copy + 'static {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription];
    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription];

    fn to_add(&self) -> *const () {
        ptr::from_ref(self).cast()
    }
}
