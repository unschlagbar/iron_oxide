use std::{
    fmt::{Debug, Formatter, Result},
    ops::Deref,
};

use crate::ui::{Ui, UiElement};

/// Handles immutable elements that can only be mutated by involving a &mut Uistate
#[derive(Clone, Copy)]
pub struct UiRef {
    inner: *mut UiElement,
}

impl UiRef {
    pub fn new(element: &mut UiElement) -> Self {
        Self { inner: element }
    }

    pub fn new_ref(element: &UiElement) -> Self {
        Self {
            inner: element as *const UiElement as *mut UiElement,
        }
    }

    #[allow(unused)]
    pub fn get_mut(mut self, ui: &mut Ui) -> &mut UiElement {
        unsafe { &mut *self.inner }
    }
}

impl Deref for UiRef {
    type Target = UiElement;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}
impl Debug for UiRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (unsafe { &*self.inner }).fmt(f)
    }
}
