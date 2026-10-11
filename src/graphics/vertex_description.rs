use std::ptr;

use pyronyx::vk;

pub trait VertexDescription: Sized + Copy + 'static {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription];
    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription];

    fn to_add(&self) -> *const () {
        ptr::from_ref(self).cast()
    }

    /// `Resources::add` drops instances that would draw nothing.
    fn is_visible(&self) -> bool {
        true
    }
}
