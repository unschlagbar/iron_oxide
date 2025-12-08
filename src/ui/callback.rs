use crate::ui::{UiRef, UiState};

use super::UiEvent;

/// ErasedFnPointer can either points to a free function or associated one that
/// `&mut self`
#[derive(Clone, Copy)]
pub struct FnPtr {
    fp: Option<fn(CallContext)>,
}

impl FnPtr {
    pub fn new(func: fn(CallContext)) -> Self {
        Self { fp: Some(func) }
    }

    pub const fn none() -> Self {
        Self { fp: None }
    }

    pub fn is_none(&self) -> bool {
        self.fp.is_none()
    }

    pub fn call(&self, context: CallContext) {
        if let Some(func) = self.fp {
            func(context)
        }
    }
}

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
