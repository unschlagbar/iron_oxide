#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    None,
    Block,
    Absolute,
    Button,
    Text,
    ScrollPanel,
}

impl ElementType {
    pub const fn has_interaction(&self) -> bool {
        match self {
            Self::Button => true,
            Self::ScrollPanel => true,
            _ => false,
        }
    }
}
