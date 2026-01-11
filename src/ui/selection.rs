use crate::ui::{UiElement, UiRef};

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
    pub fn update_ptr(&mut self, element: UiRef) {
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

    pub fn set_hover(&mut self, element: UiRef) {
        self.hovered = Some(Select::new(element));
    }

    pub fn clear_hover(&mut self) {
        self.hovered = None;
    }

    pub fn get_focused<'a>(&mut self) -> Option<&'a mut UiElement> {
        self.focused.as_mut().map(|x| x.as_mut())
    }

    pub fn set_capture(&mut self, element: UiRef) {
        self.captured = Some(Select::new(element));
    }

    pub fn clear_capture(&mut self) {
        self.captured = None;
    }

    pub fn check_removed(&mut self, id: u32) -> bool {
        let was_hover;

        if let Some(hovered) = &self.hovered
            && hovered.as_ref().id == id
        {
            self.hovered = None;
            was_hover = true;
        } else {
            was_hover = false;
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

        was_hover
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Select {
    ptr: UiRef,
    id: u32,
}

impl Select {
    pub(crate) const fn new(element: UiRef) -> Self {
        Self {
            id: element.as_ref().id,
            ptr: element,
        }
    }

    pub(crate) fn update_ptr(&mut self, element: UiRef) {
        if self.id == element.id {
            self.ptr = element;
        }
    }

    pub(crate) const fn as_mut<'b>(&mut self) -> &'b mut UiElement {
        unsafe { self.ptr.as_mut() }
    }

    pub(crate) const fn as_ref(&self) -> &UiElement {
        self.ptr.as_ref()
    }

    pub(crate) const fn as_ui_ref(&self) -> UiRef {
        self.ptr
    }
}

impl PartialEq for Select {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
