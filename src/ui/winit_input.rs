use winit::{
    event::WindowEvent,
    window::{CursorIcon, Window},
};

use crate::{
    primitives::Vec2,
    ui::{Ui, UiEvent, system::InputResult},
};

impl Ui {
    pub fn window_event(&mut self, event: &WindowEvent, window: &Window) -> InputResult {
        if matches!(
            event,
            WindowEvent::CursorMoved {
                device_id: _,
                position: _
            }
        ) || matches!(
            event,
            WindowEvent::MouseInput {
                device_id: _,
                state: _,
                button: _,
            }
        ) {
            self.cursor_icon = CursorIcon::Default;
        }

        let result = match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self.handle_input((*position).into(), UiEvent::Move),
            WindowEvent::CursorLeft { device_id: _ } => {
                self.handle_input(Vec2::new(-1, -1), UiEvent::Move)
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
            } => self.handle_input(self.cursor_pos, (*state, *button).into()),
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

        if self.is_dirty() && result.is_new() {
            window.request_redraw();
        }

        if self.cursor_icon != self.current_cursor_icon {
            window.set_cursor(self.cursor_icon);
            self.current_cursor_icon = self.cursor_icon;
        }

        result
    }
}
