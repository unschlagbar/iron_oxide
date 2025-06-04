use core::fmt::Debug;
use std::rc::Rc;
use super::{callback::ErasedFnPointer, Interaction, Style, UiElement, UiType};

#[derive(Clone)]
pub struct Button {
    pub hover_style: Style,
    pub press_style: Style,
    pub interaction: Interaction,
    pub on_press: ErasedFnPointer,
    pub before_press: ErasedFnPointer,
}

impl Button {
    pub fn new(style: Style, hover_style: Style, press_style: Style, childs: Vec<Rc<UiElement>>) -> UiElement {
        UiElement::extend(
            style,
            childs,
            UiType::Button(Self::button(hover_style, press_style))
        )
    }

    pub const fn button(hover_style: Style, press_style: Style) -> Self {
        Self { hover_style, press_style, interaction: Interaction::None, on_press: ErasedFnPointer::null(), before_press: ErasedFnPointer::null() }
    }

    pub fn on_press<S>(&mut self, struct_pointer: &mut S, fp: fn(&mut S, &mut UiElement)) {
        self.on_press = ErasedFnPointer::from_associated(struct_pointer, fp);
    }

    pub fn before_press<S>(&mut self, struct_pointer: &mut S, fp: fn(&mut S, &mut UiElement)) {
        self.before_press = ErasedFnPointer::from_associated(struct_pointer, fp);
    }
    
}

impl Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiButton").field("hover_style", &self.hover_style).field("press_style", &self.press_style).field("interaction", &self.interaction).finish()
    }
}