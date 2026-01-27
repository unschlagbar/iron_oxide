use crate::{primitives::Vec2, ui::FlexDirection};

use super::Font;

#[derive(Debug)]
pub struct BuildContext {
    /// The scale that should be applied to pixels
    pub scale_factor: f32,

    /// Size of current element
    pub element_size: Vec2<f32>,
    /// Final position of current element
    pub element_pos: Vec2<f32>,
    /// Depth
    pub z_index: i16,

    /// Available layout space given by parent (content box)
    pub available_size: Vec2<f32>,

    /// Absolute start position for children
    pub child_start_pos: Vec2<f32>,

    /// For measuring in the current flex-direction
    pub used_main: f32,
    /// For measuring the cross side (max child size)
    pub used_cross: f32,

    /// flex axis
    pub flex_direction: FlexDirection,

    font: *const Font,
}

impl BuildContext {
    pub fn default(font: &Font, parent_size: Vec2<f32>, scale_factor: f32) -> Self {
        Self {
            scale_factor,

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

    pub fn new(&self, available: Vec2<f32>, start: Vec2<f32>, dir: FlexDirection) -> Self {
        Self {
            scale_factor: self.scale_factor,

            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),
            z_index: 0,

            available_size: available,
            child_start_pos: start,

            used_main: 0.0,
            used_cross: 0.0,

            flex_direction: dir,
            font: self.font,
        }
    }

    #[inline]
    pub fn font(&self) -> &Font {
        unsafe { &*self.font }
    }

    /// Gets the remaining space
    pub fn remaining_space(&self) -> Vec2<f32> {
        match self.flex_direction {
            FlexDirection::Horizontal => self.available_size - Vec2::new(self.used_main, 0.0),
            FlexDirection::Vertical => self.available_size - Vec2::new(0.0, self.used_main),
        }
    }

    /// Gets the remaining space
    pub fn size(&self) -> Vec2<f32> {
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
    pub fn place_child(&mut self, child_size: Vec2<f32>) {
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
    pub fn pos_child(&self) -> Vec2<f32> {
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

    /// Gets the hard limited space
    pub fn hard_size(&self, size: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(
            if self.available_size.x == f32::MAX
                && matches!(self.flex_direction, FlexDirection::Vertical)
            {
                self.used_cross.max(size.x)
            } else {
                self.available_size.x
            },
            if self.available_size.y == f32::MAX
                && matches!(self.flex_direction, FlexDirection::Horizontal)
            {
                self.used_cross.max(size.y)
            } else {
                self.available_size.y
            },
        )
    }

    pub fn apply_size(&mut self, size: Vec2<f32>) {
        self.element_size = size;
    }

    pub fn apply_pos(&mut self, pos: Vec2<f32>) {
        self.element_pos = pos;
    }

    pub fn final_size(&self) -> Vec2<f32> {
        match self.flex_direction {
            FlexDirection::Horizontal => Vec2::new(self.used_main, self.used_cross),
            FlexDirection::Vertical => Vec2::new(self.used_cross, self.used_main),
        }
    }
}
