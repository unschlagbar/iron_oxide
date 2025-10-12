use std::ptr::NonNull;

use crate::ui::{ui_state::EventResult, UiElement, UiEvent, UiState};

#[derive(Default)]
pub struct Selection {
    pub hovered: Option<NonNull<UiElement>>,
    pub active_input: Option<NonNull<UiElement>>,
}
impl Selection {
    pub fn clear(&mut self) {
        self.hovered = None;
        self.active_input = None;
    }

    pub fn check(&mut self, ui: &mut UiState, event: UiEvent) -> EventResult {
        if let Some(hovered) = &mut self.hovered {
            unsafe {
                hovered.as_mut().element.interaction(hovered.as_mut(), ui, event)
            }
        } else {
            EventResult::None
        }
    }

    pub fn end(&mut self, ui: &mut UiState) -> EventResult {
        if let Some(hovered) = &mut self.hovered {
            unsafe { hovered.as_mut().element.interaction(hovered.as_mut(), ui, UiEvent::End) }
        } else {
            EventResult::None
        }
    }

    pub fn hover_id(&self) -> u32 {
        if let Some(hovered) = &self.hovered {
            unsafe { hovered.as_ref().id }
        } else {
            0
        }
    }

    pub fn get_hovered(&self) -> Option<&mut UiElement> {
        self.hovered.map(|mut x| unsafe { x.as_mut() })
    }

    pub fn set_hover(&mut self, element: &mut UiElement) {
        self.hovered = Some(NonNull::from_mut(element))
    }

    pub fn check_removed(&mut self, id: u32) {
        if let Some(hovered) = &self.hovered {
            if unsafe { hovered.as_ref().id } == id {
                self.hovered = None;
            }
        }
    }
}