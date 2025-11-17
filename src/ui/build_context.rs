use std::ptr::{self, null};

use crate::{
    primitives::Vec2,
    ui::{FlexDirection, UiElement},
};

use super::Font;

#[derive(Debug)]
pub struct BuildContext {
    /// Size of current element
    pub element_size: Vec2,
    /// Final position of current element
    pub element_pos: Vec2,

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

    pub parent: *const UiElement,
    font: *const Font,
}

impl BuildContext {
    pub fn default(font: &Font, parent_size: Vec2) -> Self {
        Self {
            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),

            available_size: parent_size,
            child_start_pos: Vec2::zero(),

            used_main: 0.0,
            used_cross: 0.0,

            flex_direction: FlexDirection::Vertical,
            parent: null(),
            font: font as _,
        }
    }

    pub fn new_from(
        parent: &Self,
        available: Vec2,
        start: Vec2,
        parent_element: &UiElement,
        dir: FlexDirection,
    ) -> Self {
        Self {
            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),

            available_size: available,
            child_start_pos: start,

            used_main: 0.0,
            used_cross: 0.0,

            flex_direction: dir,
            parent: ptr::from_ref(parent_element),
            font: parent.font,
        }
    }

    #[inline]
    pub fn font(&self) -> &Font {
        unsafe { &*self.font }
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
    pub fn pos_child(&mut self) -> Vec2 {
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
