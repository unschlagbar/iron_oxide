use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase};

use crate::ui::{UiElement, UiRef};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiEvent {
    Press,
    Release,
    TouchRelease,
    RightPress,
    RightRelease,
    Move,
    Scroll(MouseScrollDelta),
    Tick,
    HoverEnd,
    End,
    Submit,
    UnFocus,
}

impl UiEvent {
    pub fn is_release(&self) -> bool {
        matches!(self, Self::Release | Self::TouchRelease)
    }
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
    pub element_id: usize,
    pub done: bool,
    pub element: UiRef,
}

impl TickEvent {
    pub fn new(element: UiRef) -> Self {
        let element_id = element.id;
        Self {
            element_id,
            done: false,
            element,
        }
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    pub element_id: usize,
    pub element_ref: UiRef,
    pub element_name: &'static str,
    pub event: UiEvent,
    pub message: u16,
}

impl QueuedEvent {
    pub fn new(element: &UiElement, event: UiEvent, message: u16) -> Self {
        Self {
            element_id: element.id,
            element_ref: UiRef::new_ref(element),
            element_name: element.name,
            event,
            message,
        }
    }
}

#[derive(Debug, Default)]
pub struct QueuedEventHandler {
    event1: Option<QueuedEvent>,
    event2: Option<QueuedEvent>,
    first: u8,
}

impl QueuedEventHandler {
    pub fn set(&mut self, event: QueuedEvent) {
        let count = self.event1.is_some() as u8 + self.event2.is_some() as u8;

        if count == 0 {
            self.event1 = Some(event);
            self.first = 0;
        } else if count == 1 {
            if self.event1.is_none() {
                self.event1 = Some(event);
                self.first = 1;
            } else {
                self.event2 = Some(event);
                self.first = 0;
            }
        } else if self.first == 0 {
            self.first = 1;
            self.event1 = Some(event);
        } else {
            self.first = 0;
            self.event2 = Some(event);
        }
    }

    pub fn get(&mut self) -> Option<QueuedEvent> {
        if self.first == 0 {
            self.first = 1;
            self.event1.take().or_else(|| self.event2.take())
        } else {
            self.event2.take().or_else(|| self.event1.take())
        }
    }
}
