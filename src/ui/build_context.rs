use crate::{primitives::Vec2, ui::FlexDirection};

use super::Font;

#[derive(Debug)]
pub struct BuildContext {
    /// Size of current element
    pub element_size: Vec2,
    /// Final position of current element
    pub element_pos: Vec2,
    /// Depth
    pub z_index: i16,

    /// Available layout space given by parent (content box)
    pub available_size: Vec2,

    /// Absolute start position for children
    pub child_start_pos: Vec2,

    /// For measuring in the current flex-direction
    pub used_main: f32,
    /// For measuring the cross side (max child size)
    pub used_cross: f32,

    /// flex axis
    pub flex_direction: FlexDirection,

    font: *const Font,
}

impl BuildContext {
    pub fn default(font: &Font, parent_size: Vec2) -> Self {
        Self {
            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),
            z_index: 0,

            available_size: parent_size,
            child_start_pos: Vec2::zero(),

            used_main: 0.0,
            used_cross: 0.0,

            flex_direction: FlexDirection::Vertical,
            font: font as _,
        }
    }

    pub fn new_from(parent: &Self, available: Vec2, start: Vec2, dir: FlexDirection) -> Self {
        Self {
            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),
            z_index: 0,

            available_size: available,
            child_start_pos: start,

            used_main: 0.0,
            used_cross: 0.0,

            flex_direction: dir,
            font: parent.font,
        }
    }

    #[inline]
    pub fn font(&self) -> &Font {
        unsafe { &*self.font }
    }

    /// Gets the remaining space
    pub fn remaining_space(&self) -> Vec2 {
        match self.flex_direction {
            FlexDirection::Horizontal => self.available_size - Vec2::new(self.used_main, 0.0),
            FlexDirection::Vertical => self.available_size - Vec2::new(0.0, self.used_main),
        }
    }

    /// Gets the remaining space
    pub fn size(&self) -> Vec2 {
        if self.used_cross > 0.0 {
            match self.flex_direction {
                FlexDirection::Horizontal => Vec2::new(self.used_main, self.used_cross),
                FlexDirection::Vertical => Vec2::new(self.used_cross, self.used_main),
            }
        } else {
            self.available_size
        }
    }

    /// Places an element in the flow layout (similar to CSS block-level flex positioning)
    pub fn place_child(&mut self, child_size: Vec2) {
        match self.flex_direction {
            FlexDirection::Horizontal => {
                self.used_main += child_size.x;
                self.used_cross = self.used_cross.max(child_size.y);
            }
            FlexDirection::Vertical => {
                self.used_main += child_size.y;
                self.used_cross = self.used_cross.max(child_size.x);
            }
        }
    }

    /// Places an element in the flow layout (similar to CSS block-level flex positioning)
    pub fn pos_child(&self) -> Vec2 {
        match self.flex_direction {
            FlexDirection::Horizontal => {
                let x = self.child_start_pos.x + self.used_main;
                let y = self.child_start_pos.y;
                Vec2::new(x, y)
            }
            FlexDirection::Vertical => {
                let x = self.child_start_pos.x;
                let y = self.child_start_pos.y + self.used_main;
                Vec2::new(x, y)
            }
        }
    }

    pub fn apply_data(&mut self, pos: Vec2, size: Vec2) {
        self.element_pos = pos;
        self.element_size = size;
    }

    pub fn final_size(&self) -> Vec2 {
        match self.flex_direction {
            FlexDirection::Horizontal => Vec2::new(self.used_main, self.used_cross),
            FlexDirection::Vertical => Vec2::new(self.used_cross, self.used_main),
        }
    }
}
