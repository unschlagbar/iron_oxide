use std::{
    fmt::{Debug, Formatter, Result},
    ops::Deref,
    ptr::NonNull,
    slice::IterMut,
};

use crate::ui::{Ui, UiElement, widget::Widget};

/// A raw pointer to an element; it dangles once a sibling insert reallocates the parent's childs.
/// Keep a `UiRef` only into parents built once; for parents that change, use the element id.
/// With `ui_ref_check` (debug builds) every access panics if the element moved or was removed.
#[derive(Clone, Copy)]
pub struct UiRef {
    inner: NonNull<UiElement>,
    #[cfg(ui_ref_check)]
    uid: u64,
}

impl UiRef {
    pub fn new(element: &mut UiElement) -> Self {
        Self {
            #[cfg(ui_ref_check)]
            uid: element.uid,
            inner: NonNull::from_mut(element),
        }
    }

    pub fn new_ref(element: &UiElement) -> Self {
        Self {
            #[cfg(ui_ref_check)]
            uid: element.uid,
            inner: NonNull::from_ref(element),
        }
    }

    #[inline(always)]
    #[track_caller]
    fn check(&self) {
        #[cfg(ui_ref_check)]
        super::ref_check::check(self.uid, self.inner.as_ptr());
    }

    #[allow(unused)]
    #[track_caller]
    pub fn get_mut<'a>(&mut self, ui: &'a mut Ui) -> &'a mut UiElement {
        self.check();
        unsafe { self.inner.as_mut() }
    }

    #[allow(unused)]
    #[inline]
    #[track_caller]
    pub fn try_downcast_mut<'a, T: Widget>(&mut self, ui: &'a mut Ui) -> Option<&'a mut T> {
        self.check();
        unsafe { self.inner.as_mut().try_downcast_mut() }
    }

    #[allow(unused)]
    #[inline]
    #[track_caller]
    pub fn downcast_mut<'a, T: Widget>(&mut self, ui: &'a mut Ui) -> &'a mut T {
        self.check();
        unsafe { self.inner.as_mut().downcast_mut() }
    }

    #[track_caller]
    pub fn as_ref<'a>(&self) -> &'a UiElement {
        self.check();
        unsafe { self.inner.as_ref() }
    }

    #[allow(unused, clippy::missing_safety_doc)]
    #[track_caller]
    pub(crate) unsafe fn as_mut<'a>(&mut self) -> &'a mut UiElement {
        self.check();
        unsafe { self.inner.as_mut() }
    }

    #[allow(unused)]
    #[track_caller]
    pub fn childs_mut(mut self, ui: &mut Ui) -> IterMut<'_, UiElement> {
        self.get_mut(ui).childs.iter_mut()
    }
}

impl Deref for UiRef {
    type Target = UiElement;

    #[track_caller]
    fn deref(&self) -> &Self::Target {
        self.check();
        unsafe { self.inner.as_ref() }
    }
}
impl Debug for UiRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.check();
        (unsafe { self.inner.as_ref() }).fmt(f)
    }
}
