
#[derive(Clone, Copy, Debug)]
pub struct Overflow {
    pub x: OverflowAxis,
    pub y: OverflowAxis,
}

impl Overflow {
    pub const fn scroll() -> Self {
        Overflow { x: OverflowAxis::Scroll, y: OverflowAxis::Scroll }
    }

    pub const fn clip() -> Self {
        Overflow { x: OverflowAxis::Clip, y: OverflowAxis::Clip }
    }

    pub const fn hidden() -> Self {
        Overflow { x: OverflowAxis::Hidden, y: OverflowAxis::Hidden }
    }

    pub const fn visible() -> Self {
        Overflow { x: OverflowAxis::Visible, y: OverflowAxis::Visible }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum OverflowAxis {
    Visible,
    Clip,
    Hidden,
    Scroll,
}