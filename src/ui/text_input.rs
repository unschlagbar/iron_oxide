use std::rc::Rc;
use core::fmt::Debug;

use super::{Style, UiElement, UiType};

#[derive(Clone)]
pub struct TextInput {
    pub enabled_style: Style,
    pub enabled: bool,
    pub on_input: Option<Rc<dyn InputCallback>>,
}

impl TextInput {
    pub fn new(style: Style, enabled_style: Style) -> UiElement {
        UiElement::extend(
            style,
            vec![],
            UiType::TextInput(Self::text_input(enabled_style))
        )
    }

    pub const fn text_input(enabled_style: Style) -> Self {
        Self { enabled_style, enabled: false, on_input: None }
    }
}

impl Debug for TextInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput").field("enabled_style", &self.enabled_style).field("enabled", &self.enabled).field("on_input_callback", &self.on_input.is_some()).finish()
    }
}

pub trait InputCallback {
    fn call(&self, element: &TextInput, cancel: &mut bool);
}

impl<F: 'static + Fn()> InputCallback for F {
    fn call(&self, _element: &TextInput, _cancel: &mut bool) {
        self();
    }
}