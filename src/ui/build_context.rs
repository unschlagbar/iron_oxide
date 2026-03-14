use crate::{
    primitives::Vec2,
    ui::{FlexAxis, units::FlexAlign},
};

use super::Font;

#[derive(Debug, PartialEq, Eq)]
pub enum BuildPass {
    First,
    Second,
}

#[derive(Debug)]
pub struct BuildContext<'a> {
    /// The scale that should be applied to pixels
    pub scale_factor: f32,

    /// Number of childs that want to fill parent
    pub fill_sum: f32,

    /// Size of current element
    pub element_size: Vec2<f32>,
    /// Final position of current element
    pub element_pos: Vec2<f32>,
    /// Depth
    pub z_index: i16,

    /// Available layout space given by parent (content box)
    pub available_space: Vec2<f32>,

    /// Absolute start position for children
    pub child_start_pos: Vec2<f32>,

    /// For measuring in the current flex-direction
    pub used_main: f32,
    /// For measuring the cross side (max child size)
    pub used_cross: f32,

    /// For measuring in the current flex-direction
    pub predicted_main: f32,
    /// For measuring the cross side (max child size)
    pub predicted_cross: f32,

    /// flex axis
    pub flex_axis: FlexAxis,

    pub flex_start: FlexAlign,

    pub is_fill: bool,
    pub pass: BuildPass,

    pub font: &'a Font,
}

impl<'a> BuildContext<'a> {
    pub fn default(font: &'a Font, parent_size: Vec2<f32>, scale_factor: f32) -> Self {
        Self {
            scale_factor,

            fill_sum: 0.0,

            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),
            z_index: 0,

            available_space: parent_size,
            child_start_pos: Vec2::zero(),

            used_main: 0.0,
            used_cross: 0.0,

            predicted_main: 0.0,
            predicted_cross: 0.0,

            flex_axis: FlexAxis::Vertical,
            flex_start: FlexAlign::Start,
            is_fill: false,
            pass: BuildPass::First,
            font,
        }
    }

    pub fn child(
        &self,
        available: Vec2<f32>,
        start: Vec2<f32>,
        dir: FlexAxis,
        align: FlexAlign,
    ) -> Self {
        Self {
            scale_factor: self.scale_factor,

            fill_sum: 0.0,

            element_size: Vec2::zero(),
            element_pos: Vec2::zero(),
            z_index: 0,

            available_space: available,
            child_start_pos: start,

            used_main: 0.0,
            used_cross: 0.0,

            predicted_main: 0.0,
            predicted_cross: 0.0,

            flex_axis: dir,
            flex_start: align,

            is_fill: false,
            pass: BuildPass::First,

            font: self.font,
        }
    }

    pub fn next(&mut self) {
        self.pass = BuildPass::Second;

        //println!("context: {:#?}", self);
        self.predicted_main = self.used_main;
        self.predicted_cross = self.used_cross;
    }

    /// Gets the remaining space
    pub fn predicted_space(&self) -> Vec2<f32> {
        match self.flex_axis {
            FlexAxis::Horizontal => self.available_space - Vec2::new(self.predicted_main, 0.0),
            FlexAxis::Vertical => self.available_space - Vec2::new(0.0, self.predicted_main),
        }
    }

    /// Gets the remaining space
    pub fn space(&self) -> Vec2<f32> {
        match self.flex_axis {
            FlexAxis::Horizontal => self.available_space - Vec2::new(self.used_main, 0.0),
            FlexAxis::Vertical => self.available_space - Vec2::new(0.0, self.used_main),
        }
    }

    /// Places an element in the flow layout (similar to CSS block-level flex positioning)
    pub fn place(&mut self, outer_size: Vec2<f32>) {
        match self.flex_axis {
            FlexAxis::Horizontal => {
                self.used_main += outer_size.x;
                self.used_cross = self.used_cross.max(outer_size.y);
            }
            FlexAxis::Vertical => {
                self.used_main += outer_size.y;
                self.used_cross = self.used_cross.max(outer_size.x);
            }
        }
    }

    /// Places an element in the flow layout (similar to CSS block-level flex positioning)
    pub fn predict(&mut self, outer_size: Vec2<f32>) {
        match self.flex_axis {
            FlexAxis::Horizontal => {
                self.predicted_main += outer_size.x;
                self.predicted_cross = self.predicted_cross.max(outer_size.y);
            }
            FlexAxis::Vertical => {
                self.predicted_main += outer_size.y;
                self.predicted_cross = self.predicted_cross.max(outer_size.x);
            }
        }
    }

    /// Places an element in the flow layout (similar to CSS block-level flex positioning)
    pub fn pos_child(&self, flex_align: FlexAlign, size: Vec2<f32>) -> Vec2<f32> {
        match self.flex_axis {
            FlexAxis::Horizontal => match self.flex_start {
                FlexAlign::Start => {
                    let x = self.child_start_pos.x + self.used_main;
                    let y = self.child_start_pos.y - flex_align.get_pos(self.used_cross, size.y);
                    Vec2::new(x, y)
                }
                FlexAlign::End => {
                    let x =
                        self.child_start_pos.x + self.available_space.x - self.used_main - size.y;
                    let y = self.child_start_pos.y - flex_align.get_pos(self.used_cross, size.y);
                    Vec2::new(x, y)
                }
                _ => unreachable!(),
            },
            FlexAxis::Vertical => match self.flex_start {
                FlexAlign::Start => {
                    let x = self.child_start_pos.x - flex_align.get_pos(self.used_cross, size.x);
                    let y = self.child_start_pos.y + self.used_main;
                    Vec2::new(x, y)
                }
                FlexAlign::End => {
                    let x = self.child_start_pos.x - flex_align.get_pos(self.used_cross, size.x);
                    let y = self.child_start_pos.y + self.used_main + size.y;
                    Vec2::new(x, y)
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn apply_size(&mut self, size: Vec2<f32>) {
        self.element_size = size;
    }

    pub fn apply_pos(&mut self, pos: Vec2<f32>) {
        self.element_pos = pos;
    }

    pub fn final_size(&self) -> Vec2<f32> {
        match self.flex_axis {
            FlexAxis::Horizontal => Vec2::new(self.used_main, self.used_cross),
            FlexAxis::Vertical => Vec2::new(self.used_cross, self.used_main),
        }
    }

    pub fn fill_x(&mut self, weight: f32) {
        if matches!(self.flex_axis, FlexAxis::Horizontal) {
            self.fill_sum += weight;
            self.is_fill = true;
        }
    }

    pub fn fill_y(&mut self, weight: f32) {
        if matches!(self.flex_axis, FlexAxis::Vertical) {
            self.fill_sum += weight;
            self.is_fill = true;
        }
    }

    pub fn fill_size_y(&self, weight: f32) -> f32 {
        match self.flex_axis {
            FlexAxis::Horizontal => self.available_space.y,
            FlexAxis::Vertical => self.predicted_space().y * (weight / self.fill_sum),
        }
    }

    pub fn fill_size_x(&self, weight: f32) -> f32 {
        match self.flex_axis {
            FlexAxis::Horizontal => self.predicted_space().x * (weight / self.fill_sum),
            FlexAxis::Vertical => self.available_space.x,
        }
    }
}
