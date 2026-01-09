use std::ptr::NonNull;

use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase};

use crate::ui::UiElement;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiEvent {
    Press,
    Release,
    RightPress,
    RightRelease,
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

impl From<(ElementState, MouseButton)> for UiEvent {
    fn from(value: (ElementState, MouseButton)) -> Self {
        match value {
            (ElementState::Pressed, MouseButton::Right) => Self::RightPress,
            (ElementState::Released, MouseButton::Right) => Self::RightRelease,
            (ElementState::Pressed, _) => Self::Press,
            (ElementState::Released, _) => Self::Release,
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

#[derive(Debug)]
pub struct QueuedEventHandler {
    event1: Option<QueuedEvent>,
    event2: Option<QueuedEvent>,
    first: u8,
}

impl QueuedEventHandler {
    pub const fn new() -> Self {
        Self {
            event1: None,
            event2: None,
            first: 1,
        }
    }

    pub fn set(&mut self, event: QueuedEvent) {
        let count = self.event1.is_some() as u8 + self.event2.is_some() as u8;

        if count == 0 {
            self.event1 = Some(event);
            self.first = 1;
        } else if count == 1 {
            if self.event1.is_none() {
                self.event1 = Some(event);
                self.first = 2;
            } else {
                self.event2 = Some(event);
                self.first = 1;
            }
        } else if self.first == 1 {
                self.first = 2;
                self.event1 = Some(event);
            } else {
                self.first = 1;
                self.event2 = Some(event);
            }
    }

    pub fn get(&mut self) -> Option<QueuedEvent> {
        if self.first == 1 {
            self.first = 2;
            self.event1.take().or_else(|| self.event2.take())
        } else {
            self.event2.take().or_else(|| self.event1.take())
        }
    }
}
