use winit::{
    event::{MouseButton, WindowEvent},
    window::Window,
};

use crate::{
    primitives::Vec2,
    ui::{Ui, UiEvent, ui::InputResult},
};

impl Ui {
    pub fn window_event(&mut self, event: &WindowEvent, window: &Window) -> InputResult {
        let result = match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self.handle_input((*position).into(), UiEvent::Move),
            WindowEvent::CursorLeft { device_id: _ } => {
                self.handle_input(Vec2::new(-1.0, -1.0), UiEvent::Move)
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => self.handle_input(self.cursor_pos, UiEvent::Scroll(*delta)),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => match button {
                MouseButton::Left => self.handle_input(self.cursor_pos, (*state).into()),
                _ => InputResult::None,
            },
            //Todo! Implement
            WindowEvent::Touch(_touch) => InputResult::None,
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let Some(element) = self.selection.get_focused() {
                    element.handle_key(self, event)
                } else {  
                    InputResult::None
                }
            }
            _ => InputResult::None,
        };

        if self.is_dirty() {
            window.request_redraw();
        }

        result
    }
}
