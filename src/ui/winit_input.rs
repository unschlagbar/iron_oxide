use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    window::Window,
};

use crate::{
    primitives::Vec2,
    ui::{Text, Ui, UiEvent, ui::InputResult},
};

impl Ui {
    pub fn window_event(&mut self, event: &WindowEvent, window: &Window) -> InputResult {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                //self.cursor_pos = (*position).into();

                let result = self.handle_input((*position).into(), UiEvent::Move);

                if result.is_new() {
                    self.different_dirty = true;
                    window.request_redraw();
                } else if self.different_dirty && result.is_none() {
                    window.request_redraw();
                    self.different_dirty = false;
                }
                result
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                let result = self.handle_input(Vec2::new(1000.0, 1000.0), UiEvent::Move);

                if result.is_new() {
                    self.different_dirty = true;
                    window.request_redraw();
                } else if self.different_dirty && result.is_none() {
                    window.request_redraw();
                    self.different_dirty = false;
                }
                result
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                let result = self.handle_input(self.cursor_pos, UiEvent::Scroll(*delta));

                if result.is_new() {
                    self.different_dirty = true;
                    window.request_redraw();
                } else if self.different_dirty && result.is_none() {
                    window.request_redraw();
                    self.different_dirty = false;
                }
                result
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => match button {
                MouseButton::Left => {
                    let result = self.handle_input(self.cursor_pos, (*state).into());

                    if result.is_new() {
                        window.request_redraw();
                    }
                    result
                }
                _ => InputResult::None,
            },
            WindowEvent::Touch(_touch) => {
                //let cursor_pos = touch.location.into();
                //match touch.phase {
                //    TouchPhase::Started => {
                //        if self.touch_id == 0 {
                //            self.touch_id = touch.id;
                //        }
                //    }
                //    TouchPhase::Moved => (),
                //    TouchPhase::Ended | TouchPhase::Cancelled => self.touch_id = 0,
                //}
                //self.update_cursor(cursor_pos, touch.phase.into());
                InputResult::None
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if event.state == ElementState::Pressed
                    && let Some(txt) = &event.text
                {
                    if let Some(element) = self.get_hovered() {
                        if let Some(child) = element.childs.first_mut() {
                            if let Some(text) = child.downcast_mut::<Text>() {
                                text.handle_input(txt);
                            }
                        }
                    }
                    println!("{}", txt);
                }
                InputResult::None
            }
            _ => InputResult::None,
        }
    }
}
