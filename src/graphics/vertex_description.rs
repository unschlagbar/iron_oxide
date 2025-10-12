use ash::vk;

pub trait VertexDescription {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription];
    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription];
}
