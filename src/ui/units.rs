use crate::{primitives::Vec2, ui::BuildContext};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum UiUnit {
    Zero,
    Undefined,
    Auto,
    Fill,
    Px(f32),
    Relative(f32),
    RelativeHeight(f32),
    RelativeWidth(f32),
    RelativeMax(f32),
    RelativeMin(f32),
    Rem(f32),
}

impl UiUnit {
    #[inline]
    pub fn pixelx(&self, context: &BuildContext) -> f32 {
        let parent_size = context.available_size;
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => context.remaining_space().x,
            Self::Px(pixel) => *pixel * context.scale_factor,
            Self::Relative(percent) | Self::RelativeWidth(percent) => parent_size.x * percent,
            Self::RelativeHeight(percent) => parent_size.y * percent,
            Self::RelativeMax(percent) => parent_size.max() * percent,
            Self::RelativeMin(percent) => parent_size.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    #[inline]
    pub fn pixely(&self, context: &BuildContext) -> f32 {
        let parent_size = context.available_size;
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => context.remaining_space().y,
            Self::Px(pixel) => *pixel * context.scale_factor,
            Self::Relative(percent) | Self::RelativeHeight(percent) => parent_size.y * percent,
            Self::RelativeWidth(percent) => parent_size.x * percent,
            Self::RelativeMax(percent) => parent_size.max() * percent,
            Self::RelativeMin(percent) => parent_size.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    #[inline]
    pub fn py(&self, size: Vec2<f32>, scale_factor: f32) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => size.y,
            Self::Px(pixel) => *pixel * scale_factor,
            Self::Relative(percent) | Self::RelativeHeight(percent) => size.y * percent,
            Self::RelativeWidth(percent) => size.x * percent,
            Self::RelativeMax(percent) => size.max() * percent,
            Self::RelativeMin(percent) => size.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    pub fn px(&self, size: Vec2<f32>, scale_factor: f32) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => size.x,
            Self::Px(pixel) => *pixel * scale_factor,
            Self::Relative(percent) | Self::RelativeWidth(percent) => size.x * percent,
            Self::RelativeHeight(percent) => size.y * percent,
            Self::RelativeMax(percent) => size.max() * percent,
            Self::RelativeMin(percent) => size.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    pub fn px_i16(&self, size: Vec2<i16>, scale_factor: f32) -> u16 {
        match self {
            Self::Zero => 0,
            Self::Undefined => 100,
            Self::Auto => u16::MAX,
            Self::Fill => size.x as _,
            Self::Px(pixel) => (*pixel * scale_factor) as u16,
            Self::Relative(percent) | Self::RelativeWidth(percent) => {
                (size.x as f32 * percent) as u16
            }
            Self::RelativeHeight(percent) => (size.y as f32 * percent) as u16,
            Self::RelativeMax(percent) => (size.max() as f32 * percent) as u16,
            Self::RelativeMin(percent) => (size.min() as f32 * percent) as u16,
            Self::Rem(rem) => *rem as u16,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub enum Align {
    Center,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    #[default]
    TopLeft,
}

impl Align {
    #[inline]
    pub fn get_pos(&self, space: Vec2<f32>, size: Vec2<f32>, offset: Vec2<f32>) -> Vec2<f32> {
        match self {
            Align::Center => (space - size) * 0.5 + offset,
            Align::Top => Vec2::new((space.x - size.x) * 0.5 + offset.x, offset.y),
            Align::TopRight => Vec2::new(space.x - size.x - offset.x, offset.x),
            Align::Right => Vec2::new(
                space.x - size.x - offset.x,
                (space.y - size.y) * 0.5 + offset.y,
            ),
            Align::BottomRight => {
                Vec2::new(space.x - size.x - offset.x, space.y - size.y - offset.y)
            }
            Align::Bottom => Vec2::new(
                (space.x - size.x) * 0.5 + offset.x,
                space.y - size.y - offset.y,
            ),
            Align::BottomLeft => Vec2::new(offset.x, space.y - size.y - offset.y),
            Align::Left => Vec2::new(offset.x, (space.y - size.y) * 0.5 + offset.y),
            Align::TopLeft => offset,
        }
    }

    pub fn horizontal_centered(&self) -> bool {
        matches!(self, Self::Center | Self::Top | Self::Bottom)
    }

    pub fn vertical_centered(&self) -> bool {
        matches!(self, Self::Center | Self::Right | Self::Left)
    }
}
