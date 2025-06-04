
use super::{style::Style};


pub struct UiHoverBlock {
    pub style: Style,
    pub hover_style: Style,
    pub childs: Vec<UiType>,
    pub visible: bool,
    pub dragable: bool
}

impl UiHoverBlock {
    pub const fn square(style: Style, hover_style: Style, childs: Vec<UiType>) -> Self {
        Self { style, hover_style, childs, visible: true, dragable: false }
    }
}