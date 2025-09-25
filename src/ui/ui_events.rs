use winit::event::MouseScrollDelta;

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    Press,
    Release,
    Move,
    Scroll(MouseScrollDelta),
    Tick,
}
