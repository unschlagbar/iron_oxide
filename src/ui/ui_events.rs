use winit::event::{ElementState, MouseScrollDelta, TouchPhase};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiEvent {
    Press,
    Release,
    Move,
    Scroll(MouseScrollDelta),
    Tick,
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
