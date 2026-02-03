use crate::{primitives::Vec2, ui::BuildContext};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum UiUnit {
    Undefined,
    Zero,
    Px(f32),
    #[default]
    Fit,
    Fill(f32),
    Relative(f32),
    RelativeHeight(f32),
    RelativeWidth(f32),
    RelativeMax(f32),
    RelativeMin(f32),
}

impl UiUnit {
    pub const FILL: Self = Self::Relative(1.0);

    /// Gets pre values
    /// Undefined | Zero | Px(x) => 0 | x;
    /// Depends on child => f32::MIN;
    /// Depends on parent => f32::MAX;
    pub fn pre_size(&self) -> f32 {
        match self {
            Self::Fit => f32::MIN,
            Self::Undefined | Self::Zero | Self::Px(_) => 0.0,
            _ => f32::MAX,
        }
    }

    pub fn size_x(&self, context: &BuildContext) -> f32 {
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(value) => value * context.scale_factor,
            Self::Fit => context.available_size.x,
            Self::Fill(weight) => context.fill_size_x(weight),
            Self::Relative(value) | Self::RelativeWidth(value) => context.available_size.x * value,
            Self::RelativeHeight(value) => context.available_size.y * value,
            Self::RelativeMax(value) => context.available_size.max() * value,
            Self::RelativeMin(value) => context.available_size.min() * value,
        }
    }

    pub fn size_y(&self, context: &BuildContext) -> f32 {
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(value) => value * context.scale_factor,
            Self::Fit => context.available_size.y,
            Self::Fill(weight) => context.fill_size_y(weight),
            Self::Relative(value) | Self::RelativeHeight(value) => context.available_size.y * value,
            Self::RelativeWidth(value) => context.available_size.x * value,
            Self::RelativeMax(value) => context.available_size.max() * value,
            Self::RelativeMin(value) => context.available_size.min() * value,
        }
    }

    pub fn pre_size_x(&self, context: &BuildContext) -> f32 {
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(value) => value * context.scale_factor,
            Self::Fit => 0.0,
            Self::Fill(_) => panic!(),
            Self::Relative(value) | Self::RelativeWidth(value) => context.available_size.x * value,
            Self::RelativeHeight(value) => context.available_size.y * value,
            Self::RelativeMax(value) => context.available_size.max() * value,
            Self::RelativeMin(value) => context.available_size.min() * value,
        }
    }

    pub fn pre_size_y(&self, context: &BuildContext) -> f32 {
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(value) => value * context.scale_factor,
            Self::Fit => 0.0,
            Self::Fill(_) => panic!(),
            Self::Relative(value) | Self::RelativeHeight(value) => context.available_size.y * value,
            Self::RelativeWidth(value) => context.available_size.x * value,
            Self::RelativeMax(value) => context.available_size.max() * value,
            Self::RelativeMin(value) => context.available_size.min() * value,
        }
    }

    #[inline]
    pub fn pixelx(&self, context: &BuildContext) -> f32 {
        let parent_size = context.available_size;
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(pixel) => pixel * context.scale_factor,
            Self::Fit => f32::MAX,
            Self::Fill(_) => {
                panic!()
            }
            Self::Relative(percent) | Self::RelativeWidth(percent) => parent_size.x * percent,
            Self::RelativeHeight(percent) => parent_size.y * percent,
            Self::RelativeMax(percent) => parent_size.max() * percent,
            Self::RelativeMin(percent) => parent_size.min() * percent,
        }
    }

    #[inline]
    pub fn pixely(&self, context: &BuildContext) -> f32 {
        let parent_size = context.available_size;
        match *self {
            Self::Undefined => 0.0,
            Self::Zero => 0.0,
            Self::Px(pixel) => pixel * context.scale_factor,
            Self::Fit => f32::MAX,
            Self::Fill(_) => {
                panic!()
            }
            Self::Relative(percent) | Self::RelativeHeight(percent) => parent_size.y * percent,
            Self::RelativeWidth(percent) => parent_size.x * percent,
            Self::RelativeMax(percent) => parent_size.max() * percent,
            Self::RelativeMin(percent) => parent_size.min() * percent,
        }
    }

    pub fn px_i16(&self, size: Vec2<i16>, scale_factor: f32) -> u16 {
        match self {
            Self::Zero => 0,
            Self::Undefined => 100,
            Self::Fit => u16::MAX,
            Self::Fill(_) => size.x as _,
            Self::Px(pixel) => (*pixel * scale_factor) as u16,
            Self::Relative(percent) | Self::RelativeWidth(percent) => {
                (size.x as f32 * percent) as u16
            }
            Self::RelativeHeight(percent) => (size.y as f32 * percent) as u16,
            Self::RelativeMax(percent) => (size.max() as f32 * percent) as u16,
            Self::RelativeMin(percent) => (size.min() as f32 * percent) as u16,
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
