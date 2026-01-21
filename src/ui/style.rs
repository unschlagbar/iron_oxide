use super::UiUnit;
use crate::{graphics::formats::RGBA, primitives::Vec2, ui::BuildContext};

#[derive(Debug, Clone, Copy)]
pub struct UiRect {
    pub left: UiUnit,
    pub top: UiUnit,
    pub right: UiUnit,
    pub bottom: UiUnit,
}

impl UiRect {
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

    pub fn x(&self, context: &BuildContext) -> f32 {
        self.left.pixelx(context) + self.right.pixelx(context)
    }

    pub fn y(&self, context: &BuildContext) -> f32 {
        self.top.pixely(context) + self.bottom.pixely(context)
    }

    pub fn start(&self, context: &BuildContext) -> Vec2<f32> {
        Vec2::new(self.left.pixelx(context), self.top.pixely(context))
    }

    pub fn end(&self, context: &BuildContext) -> Vec2<f32> {
        Vec2::new(self.right.pixelx(context), self.bottom.pixely(context))
    }

    pub fn size(&self, context: &BuildContext) -> Vec2<f32> {
        Vec2::new(
            self.left.pixelx(context) + self.right.pixelx(context),
            self.top.pixely(context) + self.bottom.pixelx(context),
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

impl Default for UiRect {
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

pub struct Shadow {
    pub offset: (i16, i16),
    pub blur: u16,
    pub color: RGBA,
}

impl Shadow {
    pub const fn new(blur: u16, color: RGBA) -> Self {
        Self {
            offset: (0, 0),
            blur,
            color,
        }
    }
}
