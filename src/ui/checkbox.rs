use core::fmt::Debug;

use super::{callback::ErasedFnPointer, Style, UiElement, UiType};

#[derive(Clone)]
pub struct CheckBox {
    pub hover_style: Style,
    pub press_style: Style,
    pub selected: bool,
    pub pressed: bool,
    pub enabled: bool,
    pub on_enable: ErasedFnPointer,
    pub on_disable: ErasedFnPointer,
}

impl CheckBox {
    pub fn new(style: Style, hover_style: Style, press_style: Style) -> UiElement {
        UiElement::extend(
            style,
            vec![],
            UiType::CheckBox(Self::checkbox(hover_style, press_style))
        )
    }

    pub const fn checkbox(hover_style: Style, press_style: Style) -> Self {
        Self { hover_style, press_style, selected: false, pressed: false, enabled: false, on_enable: ErasedFnPointer::null(), on_disable: ErasedFnPointer::null() }
    }

    pub fn on_enable<S>(&mut self, struct_pointer: &mut S, fp: fn(&mut S, &mut UiElement)) {
        self.on_enable = ErasedFnPointer::from_associated(struct_pointer, fp);
    }

    pub fn on_disable<S>(&mut self, struct_pointer: &mut S, fp: fn(&mut S, &mut UiElement)) {
        self.on_disable = ErasedFnPointer::from_associated(struct_pointer, fp);
    }
}

impl Debug for CheckBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiButton").field("hover_style", &self.hover_style).field("press_style", &self.press_style).field("selected", &self.selected).field("pressed", &self.pressed).finish()
    }
}