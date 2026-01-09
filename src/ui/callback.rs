use winit::event::KeyEvent;

use crate::ui::{Ui, UiRef};

use super::UiEvent;

pub struct CallbackResult {
    pub rebuild: bool,
}

impl CallbackResult {
    pub const fn new(rebuild: bool) -> CallbackResult {
        CallbackResult { rebuild }
    }

    pub const fn rebuild() -> CallbackResult {
        CallbackResult { rebuild: true }
    }

    pub const fn no_rebuild() -> CallbackResult {
        CallbackResult { rebuild: false }
    }
}

pub struct ButtonContext<'a> {
    pub ui: &'a mut Ui,
    pub element: UiRef,
    pub event: UiEvent,
}

impl<'a> ButtonContext<'a> {
    pub fn new(ui: &'a mut Ui, element: UiRef, event: UiEvent) -> Self {
        Self { ui, element, event }
    }
}

pub struct TextContext<'a> {
    pub ui: &'a mut Ui,
    pub element: UiRef,
    pub event: &'a KeyEvent,
    pub ingore: bool,
    pub exit: bool,
}

impl<'a> TextContext<'a> {
    pub fn new(ui: &'a mut Ui, element: UiRef, event: &'a KeyEvent) -> Self {
        Self {
            ui,
            element,
            event,
            ingore: false,
            exit: false,
        }
    }
}
