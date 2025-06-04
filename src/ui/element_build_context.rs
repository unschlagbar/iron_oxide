use std::ptr::null;

use crate::primitives::Vec2;

use super::{Font, RawUiElement};

#[derive(Debug)]
pub struct BuildContext {
    pub element_size: Vec2,
    pub element_pos: Vec2,
    pub parent_size: Vec2,
    pub parent_pos: Vec2,
    pub line_offset: f32,
    pub start_pos: Vec2,
    pub parent: *const RawUiElement,
    pub order: u16,
    font: *const Font,
}

impl BuildContext {
    pub fn default(font: &Font, parent_size: Vec2) -> Self {
        Self {
            element_size: Vec2::default(),
            element_pos: Vec2::default(),
            parent_size,
            parent_pos: Vec2::default(),
            line_offset: 0.0,
            start_pos: Vec2::default(),
            parent: null(),
            order: 0,
            font: font as _
        }
    }

    pub fn font(&self) -> &Font {
        unsafe { &*self.font }
    }

    pub fn new_from(context: &Self, parent_size: Vec2, parent_pos: Vec2, parent: &RawUiElement) -> Self {
        Self {
            element_size: Vec2::default(),
            element_pos: Vec2::default(),
            parent_size,
            parent_pos,
            line_offset: 0.0,
            start_pos: Vec2::default(),
            parent: parent as *const RawUiElement,
            order: 0,
            font: context.font
        }
    }

    #[inline]
    pub fn fits_in_line(&mut self, pos: &mut Vec2, size: &mut Vec2) -> bool {
        if self.parent_size.x - self.start_pos.x >= size.x {
            *pos += self.start_pos;

            self.line_offset = self.line_offset.max(size.y);
            self.start_pos.x += size.x;

            return true;

        } else {
            self.start_pos.y += self.line_offset;
            pos.y += self.start_pos.y;

            self.line_offset = size.y;
            self.start_pos.x = size.x;

            return false;
        }
    }

    pub fn apply_data(&mut self, pos: Vec2, size: Vec2) {
        self.element_pos = pos;
        self.element_size = size;
    }
}