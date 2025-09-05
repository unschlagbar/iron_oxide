use ash::vk;
use std::mem::offset_of;

use super::formats::Color;

#[repr(C)]
#[derive(Default)]
pub struct UiInstance {
    pub color: Color,
    pub border_color: Color,
    pub border: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub corner: f32,
    pub z_index: f32,
}

impl VertexDescription for UiInstance {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription] =
        &[vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription] = &[
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Self, color) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Self, border_color) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, border) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, x) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, y) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, width) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, height) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 7,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, corner) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 8,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, z_index) as u32,
        },
    ];
}

#[repr(C)]
#[derive(Default)]
pub struct AtlasInstance {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub atlas_start: u32,
    pub atlas_end: u32,
    pub z_index: f32,
}

impl VertexDescription for AtlasInstance {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription] =
        &[vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription] = &[
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, x) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, y) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, width) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, height) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: vk::Format::R32_UINT,
            offset: offset_of!(Self, atlas_start) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: vk::Format::R32_UINT,
            offset: offset_of!(Self, atlas_end) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(Self, z_index) as u32,
        },
    ];
}

pub trait VertexDescription {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription];
    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription];
}
