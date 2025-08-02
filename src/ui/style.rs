use super::UiUnit;
use crate::primitives::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct OutArea {
    pub left: UiUnit,
    pub right: UiUnit,
    pub top: UiUnit,
    pub bottom: UiUnit,
}

impl OutArea {
    pub const fn new(pixel: f32) -> Self {
        Self {
            left: UiUnit::Px(pixel),
            right: UiUnit::Px(pixel),
            top: UiUnit::Px(pixel),
            bottom: UiUnit::Px(pixel),
        }
    }

    pub const fn horizontal(value: UiUnit) -> Self {
        Self {
            left: value,
            right: value,
            top: UiUnit::Zero,
            bottom: UiUnit::Zero,
        }
    }

    pub const fn vertical(value: UiUnit) -> Self {
        Self {
            left: UiUnit::Zero,
            right: UiUnit::Zero,
            top: value,
            bottom: value,
        }
    }

    pub fn x(&self, space: Vec2) -> f32 {
        self.left.pixelx(space) + self.right.pixelx(space)
    }

    pub fn y(&self, space: Vec2) -> f32 {
        self.top.pixely(space) + self.bottom.pixely(space)
    }

    pub fn start(&self, space: Vec2) -> Vec2 {
        Vec2::new(self.left.pixelx(space), self.top.pixely(space))
    }

    pub const fn zero() -> Self {
        Self {
            left: UiUnit::Zero,
            right: UiUnit::Zero,
            top: UiUnit::Zero,
            bottom: UiUnit::Zero,
        }
    }
}

impl Default for OutArea {
    fn default() -> Self {
        Self::zero()
    }
}
