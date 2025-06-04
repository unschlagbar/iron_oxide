
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    None,
    Block,
    AbsoluteLayout,
    Button,
    Text,
}