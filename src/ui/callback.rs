#![allow(dead_code)]
use core::mem::transmute;
use core::ptr::null_mut;
use std::ptr::null;

use super::{UiElement, UiEvent, UiState};

/// ErasedFnPointer can either points to a free function or associated one that
/// `&mut self`
pub struct ErasedFnPointer {
    struct_pointer: *mut (),
    fp: *const (),
    id: usize,
    ui: bool,
}

impl Copy for ErasedFnPointer {}
impl Clone for ErasedFnPointer {
    fn clone(&self) -> Self {
        *self
    }
}

impl ErasedFnPointer {
    pub fn from_associated<S>(
        struct_pointer: &mut S,
        fp: fn(&mut S, &mut UiElement),
    ) -> ErasedFnPointer {
        ErasedFnPointer {
            struct_pointer: struct_pointer as *mut S as *mut (),
            fp: fp as *const (),
            id: usize::MAX,
            ui: false,
        }
    }

    pub fn from_associated_ui<S>(
        struct_pointer: &mut S,
        fp: fn(&mut S, &mut UiState, &mut UiElement),
    ) -> ErasedFnPointer {
        ErasedFnPointer {
            struct_pointer: struct_pointer as *mut S as *mut (),
            fp: fp as *const (),
            id: usize::MAX,
            ui: true,
        }
    }

    pub fn from_associated_vars<S, T>(
        struct_pointer: &mut S,
        fp: fn(&mut S, &mut T),
    ) -> ErasedFnPointer {
        ErasedFnPointer {
            struct_pointer: struct_pointer as *mut S as *mut (),
            fp: fp as *const (),
            id: usize::MAX,
            ui: false,
        }
    }

    pub const fn from_free(fp: fn(CallContext)) -> ErasedFnPointer {
        ErasedFnPointer {
            struct_pointer: null_mut(),
            fp: fp as *const (),
            id: usize::MAX,
            ui: false,
        }
    }

    pub const fn null() -> ErasedFnPointer {
        ErasedFnPointer {
            struct_pointer: null_mut(),
            fp: null(),
            id: usize::MAX,
            ui: false,
        }
    }

    pub fn is_null(&self) -> bool {
        self.fp.is_null()
    }

    pub fn call(&self, context: CallContext) {
        if !self.fp.is_null() && self.id == usize::MAX {
            if self.struct_pointer.is_null() {
                let fp: fn(CallContext) = unsafe { transmute::<_, fn(CallContext)>(self.fp) };
                fp(context)
            } else {
                unimplemented!()
            }
        } else {
            unimplemented!()
        }
    }

    pub fn call_vars<T>(&self, vars: &mut T) {
        if !self.fp.is_null() && self.id == usize::MAX {
            if self.struct_pointer.is_null() {
                let fp: fn(&mut T) = unsafe { transmute::<_, fn(&mut T)>(self.fp) };
                fp(vars)
            } else {
                let fp = unsafe { transmute::<_, fn(_, &mut T)>(self.fp) };
                fp(self.struct_pointer, vars)
            }
        } else {
            let fp: fn(id: usize, &mut T) =
                unsafe { transmute::<_, fn(id: usize, &mut T)>(self.fp) };
            fp(self.id, vars)
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
    pub element: &'a mut UiElement,
    pub event: UiEvent,
}

impl CallContext<'_> {
    pub fn new<'a>(
        ui: &'a mut UiState,
        element: &'a mut UiElement,
        event: UiEvent,
    ) -> CallContext<'a> {
        CallContext { ui, element, event }
    }
}
