use std::ptr::NonNull;

use crate::ui::{Ui, UiElement, UiEvent, UiRef, ui::InputResult};

#[derive(Default)]
pub(crate) struct Selection {
    pub hovered: Option<Select>,
    pub focused: Option<Select>,
    pub captured: Option<Select>,
}
impl Selection {
    pub fn clear(&mut self) {
        self.hovered = None;
        self.focused = None;
        self.captured = None;
    }

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

    pub fn end(&mut self, ui: &mut Ui) -> InputResult {
        if let Some(hovered) = &mut self.hovered {
            hovered
                .as_mut()
                .widget
                .interaction(UiRef::new(hovered.as_mut()), ui, UiEvent::End)
        } else {
            InputResult::None
        }
    }

    pub fn hover_id(&self) -> u32 {
        if let Some(hovered) = &self.hovered {
            hovered.as_ref().id
        } else {
            0
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

pub(crate) struct Select {
    ptr: NonNull<UiElement>,
    id: u32,
}

impl Select {
    pub(crate) fn new(element: &UiElement) -> Self {
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

    pub(crate) fn as_mut<'b>(&mut self) -> &'b mut UiElement {
        unsafe { self.ptr.as_mut() }
    }

    pub(crate) fn as_ref(&self) -> &UiElement {
        unsafe { self.ptr.as_ref() }
    }
}
