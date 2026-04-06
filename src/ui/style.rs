use super::UiUnit;
use crate::{graphics::formats::RGBA, primitives::Vec2, ui::BuildContext};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect<T = UiUnit> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

// Methoden die nur UiUnit brauchen
impl UiRect<UiUnit> {
    pub fn start(&self, ctx: &BuildContext) -> Vec2<f32> {
        Vec2::new(self.left.autox(ctx), self.top.autoy(ctx))
    }

    pub fn end(&self, ctx: &BuildContext) -> Vec2<f32> {
        Vec2::new(self.right.pixelx(ctx), self.bottom.pixely(ctx))
    }

    pub fn size(&self, ctx: &BuildContext) -> Vec2<f32> {
        Vec2::new(
            self.left.pixelx(ctx) + self.right.pixelx(ctx),
            self.top.pixely(ctx) + self.bottom.pixely(ctx),
        )
    }

    pub const fn px(px: f32) -> Self {
        Self::all(UiUnit::Px(px))
    }

    pub const fn left_auto() -> Self {
        Self {
            left: UiUnit::Fit,
            top: UiUnit::Zero,
            right: UiUnit::Zero,
            bottom: UiUnit::Zero,
        }
    }

    pub fn top(top: UiUnit) -> Self {
        Self {
            top,
            ..Default::default()
        }
    }

    pub fn right(right: UiUnit) -> Self {
        Self {
            right,
            ..Default::default()
        }
    }

    pub fn bottom(bottom: UiUnit) -> Self {
        Self {
            bottom,
            ..Default::default()
        }
    }
    pub fn left(left: UiUnit) -> Self {
        Self {
            left,
            ..Default::default()
        }
    }
}

// Methoden für alle T: Copy
impl<T: Copy> UiRect<T> {
    pub const fn all(v: T) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    pub const fn axes(vertical: T, horizontal: T) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }
}

impl UiRect<UiUnit> {}

impl Default for UiRect<UiUnit> {
    fn default() -> Self {
        Self::all(UiUnit::Zero)
    }
}

#[macro_export]
macro_rules! u {
    (0) => {
        UiUnit::Zero
    };
    (fit) => {
        UiUnit::Fit
    };
    (fill) => {
        UiUnit::Fr(1.0)
    };
    ($v:literal px) => {
        UiUnit::Px($v as f32)
    };
    ($v:literal %) => {
        UiUnit::Pct($v as f32 / 100.0)
    };
    ($v:literal fr) => {
        UiUnit::Fr($v as f32)
    };
    ($v:literal ph) => {
        UiUnit::Ph($v as f32 / 100.0)
    };
    ($v:literal pw) => {
        UiUnit::Pw($v as f32 / 100.0)
    };
    ($v:literal) => {
        UiUnit::Px($v as f32)
    };
    // fallback: roher UiUnit-Ausdruck
    ($v:expr) => {
        $v
    };
}

// --- CSS-style rect! Macro ---
// Order like css: top, right, bottom, left
#[macro_export]
macro_rules! rect {
    ($all:tt $($unit:ident)?) => {
        iron_oxide::ui::UiRect::all(iron_oxide::u!($all $($unit)?))
    };
    ($v:tt $($vu:ident)?, $h:tt $($hu:ident)?) => {
        iron_oxide::ui::UiRect::axes(iron_oxide::u!($v $($vu)?), iron_oxide::u!($h $($hu)?))
    };
    ($t:tt $($tu:ident)?, $r:tt $($ru:ident)?, $b:tt $($bu:ident)?, $l:tt $($lu:ident)?) => {
        iron_oxide::ui::UiRect {
            top:    iron_oxide::u!($t $($tu)?),
            right:  iron_oxide::u!($r $($ru)?),
            bottom: iron_oxide::u!($b $($bu)?),
            left:   iron_oxide::u!($l $($lu)?),
        }
    };
}
#[derive(Default, Debug, Clone, Copy)]
pub enum FlexAxis {
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
