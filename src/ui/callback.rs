use crate::ui::{UiRef, UiState};

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

pub struct CallContext<'a> {
    pub ui: &'a mut UiState,
    pub element: UiRef,
    pub event: UiEvent,
}

impl CallContext<'_> {
    pub fn new<'a>(ui: &'a mut UiState, element: UiRef, event: UiEvent) -> CallContext<'a> {
        CallContext { ui, element, event }
    }
}
