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
    pub const fn px(pixel: f32) -> Self {
        Self {
            left: UiUnit::Px(pixel),
            right: UiUnit::Px(pixel),
            top: UiUnit::Px(pixel),
            bottom: UiUnit::Px(pixel),
        }
    }

    pub const fn new(data: [UiUnit; 4]) -> Self {
        Self {
            left: data[0],
            right: data[1],
            top: data[2],
            bottom: data[3],
        }
    }

    pub const fn from(pixel: &[f32]) -> Self {
        match pixel.len() {
            1 => Self::px(pixel[0]),
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

    pub const fn left(pixel: f32) -> Self {
        Self {
            left: UiUnit::Px(pixel),
            top: UiUnit::Zero,
            right: UiUnit::Zero,
            bottom: UiUnit::Zero,
        }
    }

    pub const fn top(pixel: f32) -> Self {
        Self {
            left: UiUnit::Zero,
            top: UiUnit::Px(pixel),
            right: UiUnit::Zero,
            bottom: UiUnit::Zero,
        }
    }

    pub const fn right(pixel: f32) -> Self {
        Self {
            left: UiUnit::Zero,
            top: UiUnit::Zero,
            right: UiUnit::Px(pixel),
            bottom: UiUnit::Zero,
        }
    }

    pub const fn bottom(pixel: f32) -> Self {
        Self {
            left: UiUnit::Zero,
            top: UiUnit::Zero,
            right: UiUnit::Zero,
            bottom: UiUnit::Px(pixel),
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

#[derive(Debug)]
pub struct Shadow {
    pub offset: Vec2<i16>,
    pub blur: u16,
    pub color: RGBA,
}

impl Shadow {
    pub const fn new(blur: u16, color: RGBA) -> Self {
        Self {
            offset: Vec2::new(0, 0),
            blur,
            color,
        }
    }
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset: Vec2::default(),
            blur: 0,
            color: RGBA::ZERO,
        }
    }
}
