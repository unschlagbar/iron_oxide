use crate::primitives::Vec2;

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
    pub fn pixelx(&self, space: Vec2) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => 1.77,
            Self::Px(pixel) => *pixel,
            Self::Relative(percent) | Self::RelativeWidth(percent) => space.x * percent,
            Self::RelativeHeight(percent) => space.y * percent,
            Self::RelativeMax(percent) => space.max() * percent,
            Self::RelativeMin(percent) => space.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    #[inline]
    pub fn pixely(&self, space: Vec2) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::Undefined => 100.0,
            Self::Auto => f32::MAX,
            Self::Fill => 1.77,
            Self::Px(pixel) => *pixel,
            Self::Relative(percent) | Self::RelativeHeight(percent) => space.y * percent,
            Self::RelativeWidth(percent) => space.x * percent,
            Self::RelativeMax(percent) => space.max() * percent,
            Self::RelativeMin(percent) => space.min() * percent,
            Self::Rem(rem) => *rem,
        }
    }

    pub fn child_dependent(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

#[derive(Debug, Clone, Copy)]
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
    TopLeft,
}

impl Align {
    #[inline]
    pub fn get_pos(&self, space: Vec2, size: Vec2, offset: Vec2) -> Vec2 {
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

    pub fn is_horizontal_centered(&self) -> bool {
        match self {
            Self::Center => true,
            Self::Top => true,
            Self::Bottom => true,
            _ => false,
        }
    }

    pub fn is_vertical_centered(&self) -> bool {
        match self {
            Self::Center => true,
            Self::Right => true,
            Self::Left => true,
            _ => false,
        }
    }
}
