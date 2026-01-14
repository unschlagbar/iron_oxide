use std::time::Instant;

use ash::vk::Rect2D;
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, NamedKey},
};

use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{
        Align, BuildContext, InputResult, QueuedEvent, Ressources, TextInputContext, Ui, UiElement,
        UiEvent, UiRef,
        callback::TextExitContext,
        materials::{FontInstance, MatType, UiInstance},
        text_layout::TextLayout,
        widget::Widget,
    },
};

pub struct Text {
    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub selectable: bool,
    pub cursor: Option<InputCursor>,
    pub on_input: Option<fn(&mut TextInputContext)>,
    pub on_blur: Option<fn(TextExitContext)>,

    pub dirty: bool,
    pub font_instances: Vec<FontInstance>,
}

impl Text {
    pub fn set_new(&mut self, text: String) {
        self.text = text;
        self.dirty = true;
    }

    pub fn push_text(&mut self, text: &str) {
        self.text += text;
        self.dirty = true;
    }

    pub fn focus(ui: &mut Ui, mut element: UiRef) {
        ui.set_focus(element);

        let mut_element = element.get_mut(ui);
        //mut_element.transparent = false;

        let this: &mut Self = mut_element.downcast_mut().unwrap();

        // Todo! Move this code to interaction!
        this.cursor = Some(InputCursor {
            index: this.font_instances.len(),
            start_time: Instant::now(),
            is_on: true,
        });

        ui.set_ticking(element);
    }

    pub fn unfocus(ui: &mut Ui, mut element: UiRef, reason: ExitReason) {
        let this: &mut Self = element.get_mut(ui).downcast_mut().unwrap();
        this.cursor = None;
        if let Some(on_blur) = this.on_blur {
            let cxt = TextExitContext::new(ui, element, reason);
            on_blur(cxt);
        }

        ui.remove_tick(element.id);
        ui.set_event(QueuedEvent::new(&element, UiEvent::Submit, reason as u16));
    }

    pub fn move_cursor(&mut self, offset: isize) {
        let i;
        let char_len = self.text.len();

        if let Some(cursor) = &mut self.cursor {
            if char_len == 0 {
                i = 0;
            } else {
                i = cursor.index.saturating_add_signed(offset).min(char_len);
            }
        } else {
            return;
        };

        self.cursor = Some(InputCursor {
            index: i,
            start_time: Instant::now(),
            is_on: true,
        });
    }
}

impl Widget for Text {
    fn build(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        self.dirty = false;
        self.font_instances.clear();

        let text = if self.text.is_empty() {
            "\u{200B}"
        } else {
            &self.text
        };

        let align = self.align;
        let mut offset = context.pos_child();

        let font_size = self.layout.font_size;

        let layout = self.layout.build(text, context);

        let align_size = context.size();

        if align.vertical_centered() {
            offset.y += (align_size.y - font_size * layout.lines.len() as f32).max(0.0) * 0.5;
        }

        for line in &layout.lines {
            let mut offset = offset;
            if align.horizontal_centered() {
                offset.x += (align_size.x - line.width) * 0.5;
            }

            for c in &line.content {
                self.font_instances.push(FontInstance {
                    color: self.color,
                    pos: offset + c.pos,
                    size: c.size,
                    uv_start: c.uv_start,
                    uv_size: c.uv_size,
                    z_index: context.z_index,
                });
            }
        }

        context.place_child(layout.size);
        context.apply_data(offset, layout.size);
    }

    fn instance(
        &mut self,
        element: UiRef,
        ressources: &mut Ressources,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        for inst in &self.font_instances {
            ressources.add(MatType::Font, inst, clip);
        }

        if let Some(cursor) = &self.cursor
            && cursor.is_on
        {
            let pos = if cursor.index == 0 {
                self.font_instances[0].pos
            } else if let Some(char) = self.font_instances.get(cursor.index - 1) {
                char.pos + Vec2::new(char.size.x, 0.0)
            } else {
                return clip;
            };

            let scale = self.layout.font_size * 1.2 - self.layout.font_size;
            let to_add = UiInstance {
                color: self.color,
                border_color: RGBA::ZERO,
                border: [0; 4],
                x: pos.x as _,
                y: (pos.y - scale * 0.5) as _,
                width: 2,
                height: (self.layout.font_size + scale) as _,
                corner: 0,
                z_index: element.z_index,
            };
            ressources.add(MatType::Basic, &to_add, clip);
        }

        clip
    }

    fn is_ticking(&self) -> bool {
        self.cursor.is_some()
    }

    fn tick(&mut self, _: UiRef, ui: &mut Ui) {
        if let Some(cursor) = &mut self.cursor {
            let should_be_on = cursor.start_time.elapsed().as_millis() % 1000 < 500;

            if !cursor.is_on && should_be_on {
                cursor.is_on = true;
                ui.color_changed();
            } else if cursor.is_on && !should_be_on {
                cursor.is_on = false;
                ui.color_changed();
            }
        }
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        if event == UiEvent::End && self.cursor.is_some() {
            self.cursor = None;
            Self::unfocus(ui, element, ExitReason::Submit);
        }

        InputResult::None
    }

    fn key_event(&mut self, element: UiRef, ui: &mut Ui, event: &KeyEvent) -> InputResult {
        if event.state != ElementState::Pressed {
            return InputResult::None;
        }

        if let Some(call) = self.on_input {
            let mut context = TextInputContext::new(ui, element, event);
            call(&mut context);

            if !matches!(context.submit, ExitReason::None) {
                let reason = context.submit;
                Self::unfocus(ui, element, reason);
                return InputResult::New;
            }

            if context.ingore {
                return InputResult::None;
            }
        }

        let cursor_pos = self.cursor.as_ref().unwrap().index;
        let text_len = self.text.len();

        if let Key::Named(name) = event.logical_key {
            match name {
                NamedKey::ArrowRight => {
                    self.move_cursor(1);
                    ui.layout_changed();
                }
                NamedKey::ArrowLeft => {
                    self.move_cursor(-1);
                    ui.layout_changed();
                }
                NamedKey::Backspace => {
                    if cursor_pos != 0 {
                        let start = char_to_byte(&self.text, cursor_pos - 1);
                        let end = char_to_byte(&self.text, cursor_pos);

                        self.text.replace_range(start..end, "");
                        self.dirty = true;
                        self.move_cursor(-1);
                        ui.layout_changed();
                        return InputResult::New;
                    } else {
                        return InputResult::None;
                    }
                }
                NamedKey::Delete => {
                    if text_len != 0 && cursor_pos < text_len {
                        let start = char_to_byte(&self.text, cursor_pos);
                        let end = char_to_byte(&self.text, cursor_pos + 1);

                        self.text.replace_range(start..end, "");
                        self.dirty = true;
                        ui.layout_changed();
                        return InputResult::New;
                    } else {
                        return InputResult::None;
                    }
                }
                _ => (),
            }
        }

        if let Some(text) = &event.text
            && !text.is_empty()
        {
            let idx = char_to_byte(&self.text, cursor_pos);
            self.text.insert_str(idx, text);

            self.dirty = true;
            self.move_cursor(text.chars().count() as isize);
            ui.layout_changed();
            return InputResult::New;
        }

        InputResult::New
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            text: "Text".to_string(),
            color: RGBA::WHITE,
            layout: TextLayout::default(),
            align: Align::default(),

            selectable: true,
            cursor: None,
            on_input: Some(default_on_input),
            on_blur: None,

            dirty: true,
            font_instances: Vec::new(),
        }
    }
}

pub struct InputCursor {
    /// The index into chars
    pub index: usize,
    pub start_time: Instant,
    pub is_on: bool,
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn default_on_input(ctx: &mut TextInputContext) {
    if ctx.event.logical_key == Key::Named(NamedKey::Enter) {
        ctx.submit = ExitReason::Submit;
    } else if ctx.event.logical_key == Key::Named(NamedKey::Escape) {
        ctx.submit = ExitReason::Escape
    }
}

#[derive(Clone, Copy)]
pub enum ExitReason {
    Submit,
    ClickOutside,
    Escape,
    None,
}
