use std::{
    fmt::{Debug, Formatter, Result},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

use crate::ui::{Ui, UiElement};

/// Handles immutable elements that can only be mutated by involving a &mut Uistate
#[derive(Clone, Copy)]
pub struct UiRef {
    inner: NonNull<UiElement>,
    marker: PhantomData<*mut UiElement>,
}

impl UiRef {
    pub fn new(element: &mut UiElement) -> Self {
        Self {
            inner: NonNull::from_mut(element),
            marker: PhantomData,
        }
    }

    pub fn new_ref(element: &UiElement) -> Self {
        Self {
            inner: NonNull::from_ref(element),
            marker: PhantomData,
        }
    }

    #[allow(unused)]
    pub fn get_mut<'a>(&mut self, ui: &'a mut Ui) -> &'a mut UiElement {
        unsafe { self.inner.as_mut() }
    }

    pub const fn as_ref<'a>(&self) -> &'a UiElement {
        unsafe { self.inner.as_ref() }
    }

    #[allow(unused, clippy::missing_safety_doc)]
    pub(crate) const unsafe fn as_mut<'a>(&mut self) -> &'a mut UiElement {
        unsafe { self.inner.as_mut() }
    }

    #[allow(unused)]
    pub fn childs_mut(mut self, ui: &mut Ui) -> &mut Vec<UiElement> {
        unsafe {
            let childs: &Vec<UiElement> = &self.childs;
            #[allow(invalid_reference_casting)]
            &mut *(childs as *const Vec<UiElement> as *mut Vec<UiElement>)
        }
    }
}

impl Deref for UiRef {
    type Target = UiElement;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}
impl Debug for UiRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (unsafe { self.inner.as_ref() }).fmt(f)
    }
}
