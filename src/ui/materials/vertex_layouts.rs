use ash::vk::{self, Format, VertexInputAttributeDescription, VertexInputBindingDescription};
use std::mem::offset_of;

use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct UiInstance {
    pub color: RGBA,
    pub border_color: RGBA,
    pub border: i8,
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub corner: f32,
    pub z_index: f32,
}

impl VertexDescription for UiInstance {
    const GET_BINDING_DESCRIPTION: &[VertexInputBindingDescription] =
        &[VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[VertexInputAttributeDescription] = &[
        VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: Format::R8G8B8A8_UNORM,
            offset: offset_of!(Self, color) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: Format::R8G8B8A8_UNORM,
            offset: offset_of!(Self, border_color) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: Format::R8_SINT,
            offset: offset_of!(Self, border) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R16_SINT,
            offset: offset_of!(Self, x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R16_SINT,
            offset: offset_of!(Self, y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: Format::R16_SINT,
            offset: offset_of!(Self, width) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: Format::R16_SINT,
            offset: offset_of!(Self, height) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 7,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, corner) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 8,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, z_index) as u32,
        },
    ];
}

#[derive(Debug, Clone, Copy)]
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
    const GET_BINDING_DESCRIPTION: &[VertexInputBindingDescription] =
        &[VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[VertexInputAttributeDescription] = &[
        VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, width) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, height) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R32_UINT,
            offset: offset_of!(Self, atlas_start) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: Format::R32_UINT,
            offset: offset_of!(Self, atlas_end) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: Format::R32_SFLOAT,
            offset: offset_of!(Self, z_index) as u32,
        },
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct FontInstance {
    pub color: RGBA,
    pub pos: Vec2,
    pub size: Vec2,
    pub uv_start: (u16, u16),
    pub uv_size: (u16, u16),
    pub z_index: f32,
}

impl Default for FontInstance {
    fn default() -> Self {
        Self {
            color: RGBA::WHITE,
            pos: Vec2::zero(),
            size: Vec2::zero(),
            uv_start: (0, 0),
            uv_size: (0, 0),
            z_index: 0.0,
        }
    }
}

impl VertexDescription for FontInstance {
    const GET_BINDING_DESCRIPTION: &[VertexInputBindingDescription] =
        &[VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<FontInstance>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[VertexInputAttributeDescription] = &[
        VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: Format::R8G8B8A8_UNORM,
            offset: offset_of!(FontInstance, color) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(FontInstance, pos) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(FontInstance, size) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R32_UINT,
            offset: offset_of!(FontInstance, uv_start) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R32_UINT,
            offset: offset_of!(FontInstance, uv_size) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: Format::R32_SFLOAT,
            offset: offset_of!(FontInstance, z_index) as u32,
        },
    ];
}
