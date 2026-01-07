use std::ptr::NonNull;

use winit::event::{ElementState, MouseScrollDelta, TouchPhase};

use crate::ui::UiElement;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiEvent {
    Press,
    Release,
    Move,
    Scroll(MouseScrollDelta),
    Tick,
    HoverEnd,
    End,
}

impl From<TouchPhase> for UiEvent {
    fn from(value: TouchPhase) -> Self {
        match value {
            TouchPhase::Started => Self::Press,
            TouchPhase::Moved => Self::Move,
            TouchPhase::Ended | TouchPhase::Cancelled => Self::Release,
        }
    }
}

impl From<ElementState> for UiEvent {
    fn from(value: ElementState) -> Self {
        match value {
            ElementState::Pressed => Self::Press,
            ElementState::Released => Self::Release,
        }
    }
}

#[derive(Debug)]
pub struct TickEvent {
    pub element_id: u32,
    pub done: bool,
    pub element: NonNull<UiElement>,
}

impl TickEvent {
    pub fn new(element: &UiElement) -> Self {
        let element_id = element.id;
        let element = NonNull::from_ref(element);
        Self {
            element_id,
            done: false,
            element,
        }
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    pub element_id: u32,
    pub element_name: &'static str,
    pub event: UiEvent,
    pub message: u16,
}

impl QueuedEvent {
    pub fn new(element: &UiElement, event: UiEvent, message: u16) -> Self {
        Self {
            element_id: element.id,
            element_name: element.name,
            event,
            message,
        }
    }
}
