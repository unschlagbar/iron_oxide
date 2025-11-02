use super::UiUnit;
use crate::primitives::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct OutArea {
    pub left: UiUnit,
    pub top: UiUnit,
    pub right: UiUnit,
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

    pub const fn from(pixel: &[f32]) -> Self {
        match pixel.len() {
            1 => Self::new(pixel[0]),
            2 => Self {
                left: UiUnit::Px(pixel[0]),
                top: UiUnit::Px(pixel[1]),
                right: UiUnit::Px(pixel[0]),
                bottom: UiUnit::Px(pixel[1]),
            },
            4 => Self {
                left: UiUnit::Px(pixel[0]),
                top: UiUnit::Px(pixel[1]),
                right: UiUnit::Px(pixel[2]),
                bottom: UiUnit::Px(pixel[3]),
            },
            _ => panic!("Invalid layout"),
        }
    }

    pub const fn horizontal(value: UiUnit) -> Self {
        Self {
            left: value,
            top: UiUnit::Zero,
            right: value,
            bottom: UiUnit::Zero,
        }
    }

    pub const fn vertical(value: UiUnit) -> Self {
        Self {
            left: UiUnit::Zero,
            top: value,
            right: UiUnit::Zero,
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

    pub fn end(&self, space: Vec2) -> Vec2 {
        Vec2::new(self.right.pixelx(space), self.bottom.pixely(space))
    }

    pub fn size(&self, space: Vec2) -> Vec2 {
        Vec2::new(
            self.left.pixelx(space) + self.right.pixelx(space),
            self.top.pixely(space) + self.bottom.pixelx(space),
        )
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

#[derive(Default, Debug, Clone, Copy)]
pub enum FlexDirection {
    #[default]
    Vertical,
    Horizontal,
}
