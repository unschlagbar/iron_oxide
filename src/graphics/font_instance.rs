use ash::vk;
use std::mem::offset_of;

use super::formats::Color;
use crate::{graphics::VertexDescription, primitives::Vec2};

#[derive(Debug, Clone, Copy)]
pub struct FontInstance {
    pub color: Color,
    pub pos: Vec2,
    pub size: Vec2,
    pub uv_start: (u16, u16),
    pub uv_size: (u16, u16),
    pub z_index: f32,
}

impl Default for FontInstance {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            pos: Vec2::zero(),
            size: Vec2::zero(),
            uv_start: (0, 0),
            uv_size: (0, 0),
            z_index: 0.0,
        }
    }
}

impl VertexDescription for FontInstance {
    const GET_BINDING_DESCRIPTION: &[vk::VertexInputBindingDescription] =
        &[vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<FontInstance>() as _,
            input_rate: vk::VertexInputRate::INSTANCE,
        }];

    const GET_ATTRIBUTE_DESCRIPTIONS: &[vk::VertexInputAttributeDescription] = &[
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(FontInstance, color) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R32G32_SFLOAT,
            offset: offset_of!(FontInstance, pos) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 2,
            format: vk::Format::R32G32_SFLOAT,
            offset: offset_of!(FontInstance, size) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 3,
            format: vk::Format::R32_UINT,
            offset: offset_of!(FontInstance, uv_start) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 4,
            format: vk::Format::R32_UINT,
            offset: offset_of!(FontInstance, uv_size) as u32,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 5,
            format: vk::Format::R32_SFLOAT,
            offset: offset_of!(FontInstance, z_index) as u32,
        },
    ];
}
