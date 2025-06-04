
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Interaction {
    None,
    Hover,
    Pressed,
    Dragged,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}