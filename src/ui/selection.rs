use std::ptr::NonNull;

use crate::ui::UiElement;

#[derive(Default)]
pub(crate) struct Selection {
    /// Represents the element the cursor points at
    pub hovered: Option<Select>,
    /// Receives all inputs
    pub focused: Option<Select>,
    /// Blocks all other interactions
    pub captured: Option<Select>,
}
impl Selection {
    pub fn update_ptr(&mut self, element: &UiElement) {
        if let Some(hovered) = &mut self.hovered {
            hovered.update_ptr(element);
        }

        if let Some(focused) = &mut self.focused {
            focused.update_ptr(element);
        }

        if let Some(captured) = &mut self.captured {
            captured.update_ptr(element);
        }
    }

    pub fn get_hovered(&mut self) -> Option<&mut UiElement> {
        self.hovered.as_mut().map(|x| x.as_mut())
    }

    pub fn set_hover(&mut self, element: &UiElement) {
        self.hovered = Some(Select::new(element));
    }

    pub fn clear_hover(&mut self) {
        self.hovered = None;
    }

    pub fn get_focused<'a>(&mut self) -> Option<&'a mut UiElement> {
        self.focused.as_mut().map(|x| x.as_mut())
    }

    pub fn set_capture(&mut self, element: &UiElement) {
        self.captured = Some(Select::new(element));
    }

    pub fn clear_capture(&mut self) {
        self.captured = None;
    }


    pub fn check_removed(&mut self, id: u32) {
        if let Some(hovered) = &self.hovered
            && hovered.as_ref().id == id
        {
            self.hovered = None;
        }

        if let Some(focused) = &self.focused
            && focused.as_ref().id == id
        {
            self.focused = None;
        }

        if let Some(captured) = &self.captured
            && captured.as_ref().id == id
        {
            self.captured = None;
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Select {
    ptr: NonNull<UiElement>,
    id: u32,
}

impl Select {
    pub(crate) const fn new(element: &UiElement) -> Self {
        Self {
            ptr: NonNull::from_ref(element),
            id: element.id,
        }
    }

    pub(crate) fn update_ptr(&mut self, element: &UiElement) {
        if self.id == element.id {
            self.ptr = NonNull::from_ref(element);
        }
    }

    pub(crate) const fn as_mut<'b>(&mut self) -> &'b mut UiElement {
        unsafe { self.ptr.as_mut() }
    }

    pub(crate) const fn as_ref(&self) -> &UiElement {
        unsafe { self.ptr.as_ref() }
    }
}

impl PartialEq for Select {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
