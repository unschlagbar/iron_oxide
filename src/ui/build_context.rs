use std::ptr::{self, null};

use crate::{
    primitives::Vec2,
    ui::{FlexDirection, UiElement},
};

use super::Font;

#[derive(Debug)]
pub struct BuildContext {
    pub element_size: Vec2,
    pub element_pos: Vec2,

    pub available_size: Vec2,
    pub child_start_pos: Vec2,
    pub line_offset: f32,
    pub used_space: Vec2,
    pub flex_direction: FlexDirection,
    pub parent: *const UiElement,
    font: *const Font,
}

impl BuildContext {
    pub fn default(font: &Font, parent_size: Vec2) -> Self {
        Self {
            element_size: Vec2::default(),
            element_pos: Vec2::default(),
            available_size: parent_size,
            child_start_pos: Vec2::default(),
            line_offset: 0.0,
            used_space: Vec2::default(),
            flex_direction: FlexDirection::Vertical,
            parent: null(),
            font: font as _,
        }
    }

    pub fn font(&self) -> &Font {
        unsafe { &*self.font }
    }

    pub fn new_from(
        context: &Self,
        available_size: Vec2,
        child_start_pos: Vec2,
        parent: &UiElement,
        flex_direction: FlexDirection,
    ) -> Self {
        Self {
            element_size: Vec2::default(),
            element_pos: Vec2::default(),
            available_size,
            child_start_pos,
            line_offset: 0.0,
            used_space: Vec2::default(),
            flex_direction,
            parent: ptr::from_ref(parent),
            font: context.font,
        }
    }

    #[inline]
    pub fn fits_in_line(&mut self, pos: &mut Vec2, size: Vec2) -> bool {
        match self.flex_direction {
            FlexDirection::Horizontal => {
                if self.available_size.x - self.used_space.x >= size.x {
                    *pos += self.used_space;

                    self.line_offset = self.line_offset.max(size.y);
                    self.used_space.x += size.x;

                    true
                } else {
                    self.used_space.y += self.line_offset;
                    pos.y += self.used_space.y;

                    self.line_offset = size.y;
                    self.used_space.x = size.x;

                    false
                }
            }
            FlexDirection::Vertical => {
                if self.available_size.y - self.used_space.y >= size.y {
                    *pos += self.used_space;

                    self.line_offset = self.line_offset.max(size.x);
                    self.used_space.y += size.y;

                    true
                } else {
                    self.used_space.x += self.line_offset;
                    pos.x += self.used_space.x;

                    self.line_offset = size.x;
                    self.used_space.y = size.y;

                    false
                }
            }
        }
    }

    pub fn apply_data(&mut self, pos: Vec2, size: Vec2) {
        self.element_pos = pos;
        self.element_size = size;
    }
}
