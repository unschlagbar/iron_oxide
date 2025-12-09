use std::{
    fmt::{Debug, Formatter, Result},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::ui::{UiElement, UiState};

/// Handles immutable elements that can only be mutated by involving a &mut Uistate
#[derive(Clone, Copy)]
pub struct UiRef {
    inner: NonNull<UiElement>,
}

impl UiRef {
    pub fn new(element: &UiElement) -> Self {
        Self {
            inner: NonNull::from_ref(element),
        }
    }

    #[allow(unused)]
    pub fn get_mut(mut self, ui: &mut UiState) -> &mut UiElement {
        unsafe { self.inner.as_mut() }
    }
}

impl Deref for UiRef {
    type Target = UiElement;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl DerefMut for UiRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

impl Debug for UiRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        unsafe { self.inner.as_ref().fmt(f) }
    }
}
