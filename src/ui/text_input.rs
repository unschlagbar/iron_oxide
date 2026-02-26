use std::time::Instant;

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
        materials::{AtlasInstance, MatType, UiInstance},
        system::KeyModifiers,
        text_layout::TextLayout,
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
    pub draw_data: Vec<AtlasInstance>,
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
            draw_data: text.draw_data,
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
        self.cursor = Some(InputCursor {
            index: self.draw_data.len(),
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
        } else if this.selection.is_some() {
            ui.color_changed();
        }
        this.cursor = None;
        this.selection = None;

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

    pub fn try_select(&mut self, ui: &mut Ui) {
        if self.text.is_empty() {
            return;
        }

        let cursor = self.cursor.as_ref().unwrap();
        let mut selection = if let Some(selection) = &self.selection {
            *selection
        } else {
            Selection::start(cursor.index)
        };

        let start = cursor.index;
        let start_pos;
        let line_height;

        if start == self.draw_data.len() {
            let inst = &self.draw_data[start - 1];
            start_pos = inst.pos + Vec2::new(inst.size.x, 0.0);
            line_height = inst.size.y;
        } else {
            let inst = &self.draw_data[start];
            start_pos = inst.pos;
            line_height = inst.size.y;
        }

        let to_check;
        let mut i_offset = 0;
        let cursor_pos = ui.cursor_pos.into_f32();

        if start_pos.y <= cursor_pos.y && start_pos.y + line_height >= cursor_pos.y {
            if start_pos.x < cursor_pos.x {
                to_check = &self.draw_data[start..];
                i_offset = start;
            } else {
                to_check = &self.draw_data[..start];
            }
        } else if start_pos.y < cursor_pos.y {
            to_check = &self.draw_data[start..];
            i_offset = start;
        } else {
            to_check = &self.draw_data[..start];
        }
        let mut most_end_char = 0;

        for char in to_check {
            let y_in = cursor_pos.y >= char.pos.y && cursor_pos.y <= char.pos.y + char.size.y;
            let x_in = cursor_pos.x >= char.pos.x && cursor_pos.x <= char.pos.x + char.size.x;

            if y_in && x_in {
                // If cursor is on the right half of the glyph, treat it as selecting the next
                // character (i_offset + 1). This makes selection feel natural when the cursor
                // is over more than half of a character's width.
                if cursor_pos.x >= char.pos.x + char.size.x * 0.5 {
                    most_end_char = i_offset + 1;
                } else {
                    most_end_char = i_offset;
                }
                break;
            } else if y_in {
            }

            i_offset += 1;
        }

        selection.update(most_end_char);

        if let Some(select) = &mut self.selection {
            select.range = selection.range
        } else if selection.anchor != selection.range {
            self.selection = Some(selection);
        }

        ui.color_changed();
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
            for (i, glyph) in self.draw_data.iter().enumerate() {
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
            new_i = self.draw_data.len() as isize;
        }

        if new_i != cursor_i {
            self.move_cursor(new_i - cursor_i);
            ui.color_changed();
        }
    }
}

impl Widget for TextInput {
    fn build_layout(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        self.draw_data.clear();

        let mut offset = context.pos_child(FlexAlign::default(), Vec2::zero());
        let align_size = context.size();

        let font = self.layout.font.as_ref().unwrap_or(&context.font);
        let scale = self.layout.font_size * context.scale_factor / font.size;
        let line_height = font.line_height * scale;

        context.place_child(context.element_size);

        let lines = self.layout.lines.len() as f32;
        if self.align.vertical_centered() {
            offset.y += (align_size.y - line_height * lines).max(0.0) * 0.5;
        }

        context.apply_pos(offset);

        for line in &self.layout.lines {
            let mut offset = offset;
            if self.align.horizontal_centered() {
                offset.x += (align_size.x - line.width) * 0.5;
            }

            for c in &self.layout.glyphs[line.start..line.end] {
                self.draw_data.push(AtlasInstance {
                    color: self.color,
                    pos: offset + c.pos,
                    size: c.size,
                    uv_start: c.uv_start.into(),
                    uv_size: c.uv_size.into(),
                });
            }
        }
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let size = Vec2::new(context.fill_size_x(1.0), self.layout.size.y);
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

        self.layout.build(text, context);
        context.predict_child(Vec2::new(0.0, self.layout.size.y));
    }

    fn draw_data(&mut self, _element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {
        let mat = if let Some(font) = &self.layout.font {
            if font.bitmap {
                MatType::Bitmap
            } else {
                MatType::MSDF
            }
        } else {
            MatType::Bitmap
        };
        ressources.add_slice(mat, &self.draw_data, info);

        if let Some(selection) = &self.selection {
            let (start, end) = selection.range();

            if start != end {
                let start_pos = self.draw_data[start].pos;
                let end_pos = if end == self.draw_data.len() {
                    self.draw_data[end - 1].pos + Vec2::new(self.draw_data[start].size.x, 0.0)
                } else {
                    self.draw_data[end].pos
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
                self.draw_data[0].pos
            } else if let Some(char) = self.draw_data.get(cursor.index - 1) {
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
                println!("unfocus submit");
                return InputResult::None;
            }
            UiEvent::Press => {
                ui.selection.set_capture(element);
                if self.focus_on_click && self.cursor.is_none() {
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
                self.try_select(ui);
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
            draw_data: Vec::new(),
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

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub range: usize,
    pub anchor: usize,
}

impl Selection {
    pub fn start(index: usize) -> Self {
        Self {
            range: index,
            anchor: index,
        }
    }

    pub fn update(&mut self, index: usize) {
        self.range = index;
    }

    pub fn range(&self) -> (usize, usize) {
        if self.anchor <= self.range {
            (self.anchor, self.range)
        } else {
            (self.range, self.anchor)
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
