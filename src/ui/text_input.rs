use std::{ops::Range, time::Instant};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, NamedKey},
    window::CursorIcon,
};

use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, InputResult, QueuedEvent, Text, TextInputContext, Ui,
        UiElement, UiEvent, UiRef,
        callback::TextExitContext,
        materials::{FontInstance, MatType, UiInstance},
        system::KeyModifiers,
        text_layout::{LayoutText, TextLayout},
        units::FlexAlign,
        widget::Widget,
    },
};

pub struct TextInput {
    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub selectable: bool,
    pub focus_on_click: bool,
    pub cursor: Option<InputCursor>,
    pub selection: Option<Selection>,

    pub on_input: Option<fn(&mut TextInputContext)>,
    pub on_blur: Option<fn(TextExitContext)>,

    pub dirty: bool,
    pub build_layout: LayoutText,
    pub font_instances: Vec<FontInstance>,
}

impl TextInput {
    pub fn from(text: Text) -> Self {
        Self {
            text: text.text,
            color: text.color,
            layout: text.layout,
            align: text.align,
            selectable: text.selectable,
            focus_on_click: false,
            cursor: text.cursor,
            selection: None,
            on_input: Some(default_on_input),
            on_blur: None,
            dirty: false,
            build_layout: LayoutText::default(),
            font_instances: text.font_instances,
        }
    }
    pub fn set_new(&mut self, text: String) {
        self.text = text;
        self.dirty = true;
    }

    pub fn push_text(&mut self, text: &str) {
        self.text += text;
        self.dirty = true;
    }

    pub fn focus(ui: &mut Ui, element: UiRef) {
        ui.set_focus(element);
        ui.set_ticking(element);
    }

    pub fn set_cursor(&mut self) {
        // Todo! Move this code to interaction!
        self.cursor = Some(InputCursor {
            index: self.font_instances.len(),
            start_time: Instant::now(),
            is_on: true,
        });
    }

    pub fn unfocus(ui: &mut Ui, mut element: UiRef, reason: ExitReason) {
        let this: &mut Self = unsafe { element.as_mut().downcast_mut().unwrap() };

        if let Some(cursor) = &this.cursor
            && cursor.is_on
        {
            ui.color_changed();
        }
        this.cursor = None;

        if let Some(on_blur) = this.on_blur {
            let cxt = TextExitContext::new(ui, element, reason);
            on_blur(cxt);
        }

        ui.remove_tick(element.id);
        let event = if matches!(reason, ExitReason::Submit) {
            UiEvent::Submit
        } else {
            UiEvent::UnFocus
        };
        ui.set_event(QueuedEvent::new(&element, event, reason as u16));
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

    pub fn try_select(&mut self, _ui: &mut Ui) {
        let cursor = self.cursor.as_ref().unwrap();

        if self.selection.is_none() {
            self.selection = Some(Selection::start(cursor.index))
        }
        //let start_pos = self.font_instances[cursor.index].pos;
    }

    pub fn point_cursor(&mut self, ui: &mut Ui) {
        let cursor_pos: Vec2<f32> = ui.cursor_pos.into_f32();
        let margin = self.layout.font_size / 8.0;

        let cursor_i = if let Some(cursor) = &self.cursor {
            cursor.index as isize
        } else {
            panic!()
        };
        let mut new_i = isize::MAX;

        if self.text.is_empty() {
            new_i = 0;
        } else {
            for (i, glyph) in self.font_instances.iter().enumerate() {
                if cursor_pos >= glyph.pos - margin && cursor_pos <= glyph.pos + glyph.size + margin
                {
                    if glyph.pos.x + glyph.size.x * 0.5 <= cursor_pos.x {
                        new_i = i as isize + 1;
                    } else {
                        new_i = i as isize;
                    }
                    break;
                }
            }
        }

        if new_i == isize::MAX {
            new_i = self.font_instances.len() as isize;
        }

        if new_i != cursor_i {
            self.move_cursor(new_i - cursor_i);
            ui.color_changed();
        }
    }
}

impl Widget for TextInput {
    fn build_layout(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        self.font_instances.clear();

        let mut offset = context.pos_child(FlexAlign::default(), Vec2::zero());
        let align_size = context.size();
        let font_size = self.layout.font_size * context.scale_factor;

        context.place_child(context.element_size);

        let lines = self.build_layout.lines.len() as f32;
        if self.align.vertical_centered() {
            offset.y += (align_size.y - font_size * lines).max(0.0) * 0.5;
        }

        context.apply_pos(offset);

        for line in &self.build_layout.lines {
            let mut offset = offset;
            if self.align.horizontal_centered() {
                offset.x += (align_size.x - line.width) * 0.5;
            }

            for c in &line.content {
                self.font_instances.push(FontInstance {
                    color: self.color,
                    pos: offset + c.pos,
                    size: c.size,
                    uv_start: c.uv_start,
                    uv_size: c.uv_size,
                });
            }
        }
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let size = Vec2::new(context.fill_size_x(1.0), self.build_layout.size.y);
        context.place_child(size);
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        self.dirty = false;

        context.fill_x(1.0);

        let text = if self.text.is_empty() {
            "\u{200B}"
        } else {
            &self.text
        };

        self.build_layout = self.layout.build(text, context);
        context.predict_child(Vec2::new(0.0, self.build_layout.size.y));
    }

    fn draw_data(&mut self, _element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {
        ressources.add_slice(MatType::Font, &self.font_instances, info);

        if let Some(selection) = &self.selection {
            let start = selection.range.start;
            let end = selection.range.end;

            if start != end {
                let start_pos = self.font_instances[start].pos;
                let end_pos = if end == self.font_instances.len() {
                    self.font_instances[end - 1].pos
                        + Vec2::new(self.font_instances[start].size.x, 0.0)
                } else {
                    self.font_instances[end].pos
                };

                let to_add = UiInstance {
                    color: RGBA::rgba(0, 255, 0, 150),
                    border_color: RGBA::ZERO,
                    border: [0; 4],
                    pos: Vec2::new(start_pos.x as i16, start_pos.y as i16),
                    size: Vec2::new(
                        (end_pos.x - start_pos.x) as i16,
                        self.layout.font_size as i16,
                    ),
                    corner: 0,
                };
                ressources.add(MatType::Basic, to_add, info);
            }
        } else if let Some(cursor) = &self.cursor
            && cursor.is_on
        {
            let pos = if cursor.index == 0 {
                self.font_instances[0].pos
            } else if let Some(char) = self.font_instances.get(cursor.index - 1) {
                char.pos + Vec2::new(char.size.x, 0.0)
            } else {
                return;
            };

            let font_size = self.layout.font_size;
            let scale = font_size * 1.2 - font_size;

            let to_add = UiInstance {
                color: self.color,
                border_color: RGBA::ZERO,
                border: [0; 4],
                pos: Vec2::new(pos.x as i16, (pos.y - scale * 0.5) as i16),
                size: Vec2::new(
                    2 * info.scale_factor as i16,
                    (font_size * info.scale_factor + scale) as i16,
                ),
                corner: 0,
            };
            ressources.add(MatType::Basic, to_add, info);
        }
    }

    fn is_ticking(&self) -> bool {
        self.cursor.is_some() && self.selection.is_none()
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
        match event {
            UiEvent::End if self.cursor.is_some() => {
                Self::unfocus(ui, element, ExitReason::Submit);
                return InputResult::None;
            }
            UiEvent::Press => {
                if self.cursor.is_some() {
                    ui.selection.set_capture(element);
                } else if self.focus_on_click {
                    Self::focus(ui, element);
                    self.set_cursor();
                    ui.color_changed();
                }
                self.point_cursor(ui);
                if self.selection.is_some() && !ui.modifiers.contains(KeyModifiers::Shift) {
                    self.selection = None;
                }
            }
            UiEvent::Move if ui.selection.is_captured(element) => {
                if self.selection.is_none() {
                    self.selection = Some(Selection::start(self.cursor.as_ref().unwrap().index));
                    self.selection.as_mut().unwrap().update(0);
                    ui.color_changed();
                }
                //Handle Drag
                println!("drag")
            }
            _ => (),
        };

        ui.cursor_icon = CursorIcon::Text;
        InputResult::New
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
                ui.selection.focused = None;
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

impl Default for TextInput {
    fn default() -> Self {
        Self {
            text: "Text".to_string(),
            color: RGBA::grey(220),
            layout: TextLayout::default(),
            align: Align::default(),

            selectable: true,
            focus_on_click: true,
            cursor: None,
            selection: None,

            on_input: Some(default_on_input),
            on_blur: None,

            dirty: true,
            build_layout: LayoutText::default(),
            font_instances: Vec::new(),
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug, Default)]
pub struct Selection {
    pub range: Range<usize>,
    pub anchor: usize,
}

impl Selection {
    pub fn start(index: usize) -> Self {
        Self {
            range: index..index,
            anchor: index,
        }
    }

    pub fn update(&mut self, index: usize) {
        if index < self.anchor {
            self.range = index..self.anchor;
        } else {
            self.range = self.anchor..index;
        }
    }
}

#[derive(Clone, Copy)]
pub enum ExitReason {
    Submit,
    ClickOutside,
    Escape,
    None,
}
