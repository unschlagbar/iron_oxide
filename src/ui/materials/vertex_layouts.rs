use ash::vk::{self, Format, VertexInputAttributeDescription, VertexInputBindingDescription};
use std::mem::offset_of;

use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
};

#[repr(align(4))]
#[derive(Debug, Default, Clone, Copy)]
pub struct UiInstance {
    pub color: RGBA,
    pub border_color: RGBA,
    pub border: [u8; 4],
    pub pos: Vec2<i16>,
    pub size: Vec2<i16>,
    pub corner: u16,
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
            format: Format::R8G8B8A8_UINT,
            offset: offset_of!(Self, border) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R16_SINT,
            offset: offset_of!(Self, pos.x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R16_SINT,
            offset: offset_of!(Self, pos.y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: Format::R16_SINT,
            offset: offset_of!(Self, size.x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: Format::R16_SINT,
            offset: offset_of!(Self, size.y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 7,
            format: Format::R16_UINT,
            offset: offset_of!(Self, corner) as u32,
        },
    ];
}

#[repr(align(4))]
#[derive(Debug, Clone, Copy)]
pub struct AtlasInstance {
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub color: RGBA,
    pub uv_start: Vec2<u16>,
    pub uv_size: Vec2<u16>,
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
            format: Format::R8G8B8A8_UNORM,
            offset: offset_of!(FontInstance, color) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(Self, pos) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(Self, size) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R32_UINT,
            offset: offset_of!(Self, uv_start) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R32_UINT,
            offset: offset_of!(Self, uv_size) as u32,
        },
    ];
}

#[repr(align(4))]
#[derive(Debug, Clone, Copy)]
pub struct FontInstance {
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub color: RGBA,
    pub uv_start: (u16, u16),
    pub uv_size: (u16, u16),
}

impl Default for FontInstance {
    fn default() -> Self {
        Self {
            color: RGBA::WHITE,
            pos: Vec2::zero(),
            size: Vec2::zero(),
            uv_start: (0, 0),
            uv_size: (0, 0),
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
    ];
}

#[repr(align(4))]
#[derive(Debug, Clone, Copy)]
pub struct ShadowInstance {
    pub pos: Vec2<i16>,
    pub size: Vec2<i16>,
    pub blur: u16,
    pub corner: u16,
    pub color: RGBA,
}

impl VertexDescription for ShadowInstance {
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
            format: Format::R16_SINT,
            offset: offset_of!(Self, pos.x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: Format::R16_SINT,
            offset: offset_of!(Self, pos.y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: Format::R16_SINT,
            offset: offset_of!(Self, size.x) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: Format::R16_SINT,
            offset: offset_of!(Self, size.y) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: Format::R16_UINT,
            offset: offset_of!(Self, blur) as u32,
        },
        VertexInputAttributeDescription {
            binding: 0,
            location: 6,
            format: Format::R16_UINT,
            offset: offset_of!(Self, corner) as u32,
        },
    ];
}
