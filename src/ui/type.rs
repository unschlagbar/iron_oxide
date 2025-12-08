#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    None,
    Block,
    Absolute,
    Button,
    Text,
    TextInput,
    ScrollPanel,
    Image,
}

impl ElementType {
    pub const fn has_interaction(&self) -> bool {
        matches!(self, Self::Button | Self::ScrollPanel | Self::TextInput)
    }
}
