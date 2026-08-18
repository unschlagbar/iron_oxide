use std::{range::Range, time::Instant};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, NamedKey},
    window::CursorIcon,
};

use crate::{
    graphics::{Resources, formats::RGBA},
    hex_rgba,
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, InputResult, QueuedEvent, Text, TextInputContext, Ui,
        UiElement, UiEvent, UiRef,
        callback::StateChangeCtx,
        materials::{MSDFVertex, MatType, UiInstance},
        system::KeyModifiers,
        text_layout::TextLayout,
        units::FlexAlign,
        widget::Widget,
    },
};

pub struct TextInput {
    pub placeholder: &'static str,
    pub placeholder_color: RGBA,

    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub selectable: bool,
    pub focus_on_click: bool,
    pub cursor: Option<InputCursor>,
    pub selection: Option<Selection>,

    pub message: u16,
    pub on_input: Option<fn(&mut TextInputContext)>,
    pub state_change: Option<fn(StateChangeCtx)>,

    pub dirty: bool,
}

impl TextInput {
    pub fn from(text: Text) -> Self {
        Self {
            placeholder: "Text",
            placeholder_color: RGBA::grey(150),

            text: text.text,
            color: text.color,
            layout: text.layout,
            align: text.align,
            selectable: text.selectable,
            focus_on_click: false,
            cursor: None,
            selection: None,

            message: 0,
            on_input: Some(default_on_input),
            state_change: None,
            dirty: false,
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

    pub fn focus(&mut self, ui: &mut Ui, element: UiRef) {
        if let Some(state_change) = self.state_change {
            let cxt = StateChangeCtx::new(ui, element, StateChange::Focus);
            state_change(cxt);
        }
        ui.set_focus(element);
        ui.set_ticking(element);
    }

    fn set_cursor(&mut self, index: usize) {
        self.cursor = Some(InputCursor {
            index,
            start_time: Instant::now(),
            is_on: true,
        });
    }

    pub fn set_new_cursor(&mut self) {
        let index = if self.text.is_empty() {
            0
        } else {
            self.layout.glyphs.len()
        };
        self.set_cursor(index);
    }

    pub fn unfocus(ui: &mut Ui, mut element: UiRef, change: StateChange) {
        let this: &mut Self = unsafe { element.as_mut().downcast_mut() };

        if let Some(cursor) = &this.cursor
            && cursor.is_on
        {
            ui.color_changed();
        } else if this.selection.is_some() {
            ui.color_changed();
        }
        this.cursor = None;
        this.selection = None;

        if let Some(state_change) = this.state_change {
            let cxt = StateChangeCtx::new(ui, element, change);
            state_change(cxt);
        }

        ui.remove_tick(element.id);
        let event = if matches!(change, StateChange::Submit) {
            UiEvent::Submit
        } else {
            UiEvent::UnFocus
        };

        ui.set_event(QueuedEvent::new(&element, event, this.message));
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

        self.set_cursor(i);
    }

    pub fn try_select(&mut self, ui: &mut Ui) {
        if self.text.is_empty() {
            return;
        }

        let cursor_pos = ui.cursor_pos.into_f32();
        let new_range = self.hit_test(cursor_pos);
        let anchor = self
            .selection
            .map(|s| s.anchor)
            .unwrap_or_else(|| self.cursor.as_ref().unwrap().index);

        if let Some(sel) = &mut self.selection {
            sel.range = new_range;
        } else if anchor != new_range {
            self.selection = Some(Selection {
                anchor,
                range: new_range,
            });
        }

        ui.color_changed();
    }

    pub fn point_cursor(&mut self, ui: &mut Ui) {
        let cursor_pos: Vec2<f32> = ui.cursor_pos.into_f32();
        let new_i = self.hit_test(cursor_pos);

        let cursor_i = self.cursor.as_ref().unwrap().index as isize;

        if new_i as isize != cursor_i {
            self.move_cursor(new_i as isize - cursor_i);
            ui.color_changed();
        }
    }

    /// Returns the glyph index closest to `cursor_pos` in screen space.
    /// Uses glyph bounding boxes: finds the line whose y-range contains the cursor (or
    /// the nearest line), then snaps to line.end/start when the cursor is in a gap between
    /// lines, and finally picks the insertion point by x-midpoint within the line.
    pub fn hit_test(&self, cursor_pos: Vec2<f32>) -> usize {
        let glyphs = &self.layout.glyphs;
        if glyphs.is_empty() || self.layout.lines.is_empty() || self.text.is_empty() {
            return 0;
        }

        let mut best_line = 0;
        let mut best_dist = f32::MAX;

        for (i, line) in self.layout.lines.iter().enumerate() {
            if line.start >= glyphs.len() {
                continue;
            }
            let g = &glyphs[line.start];
            let top = g.pos.y;
            let bottom = g.pos.y + g.size.y;

            if cursor_pos.y >= top && cursor_pos.y <= bottom {
                best_line = i;
                break;
            }
            let dist = if cursor_pos.y < top {
                top - cursor_pos.y
            } else {
                cursor_pos.y - bottom
            };
            if dist < best_dist {
                best_dist = dist;
                best_line = i;
            }
        }

        let line = &self.layout.lines[best_line];
        let line_end = line.end.min(glyphs.len());

        if line.start >= glyphs.len() {
            return glyphs.len();
        }
        let g = &glyphs[line.start];

        // Snap to line boundaries when cursor is in a vertical gap
        if cursor_pos.y > g.pos.y + g.size.y {
            return line_end;
        }
        if cursor_pos.y < g.pos.y {
            return line.start;
        }

        for i in line.start..line_end {
            if cursor_pos.x < glyphs[i].pos.x + glyphs[i].size.x * 0.5 {
                return i;
            }
        }
        line_end
    }

    /// Extends or starts a selection by `offset` chars from the current cursor position.
    pub fn extend_selection(&mut self, offset: isize) {
        let cursor_i = self.cursor.as_ref().unwrap().index;
        let anchor = self.selection.map(|s| s.anchor).unwrap_or(cursor_i);
        let char_len = self.text.chars().count();
        let new_range = (cursor_i as isize + offset).clamp(0, char_len as isize) as usize;

        self.selection = if anchor != new_range {
            Some(Selection {
                anchor,
                range: new_range,
            })
        } else {
            None
        };
        self.move_cursor(offset);
    }

    /// Extends or starts a selection from the current cursor to `index`.
    pub fn extend_selection_to(&mut self, index: usize) {
        let cursor_i = self.cursor.as_ref().unwrap().index;
        let anchor = self.selection.map(|s| s.anchor).unwrap_or(cursor_i);
        self.selection = if anchor != index {
            Some(Selection {
                anchor,
                range: index,
            })
        } else {
            None
        };

        if self.cursor.is_some() {
            self.set_cursor(index);
        }
    }

    /// Returns the glyph index of the start of the line containing `cursor_i`.
    fn line_start_for(&self, cursor_i: usize) -> usize {
        for line in &self.layout.lines {
            if cursor_i <= line.end {
                return line.start;
            }
        }
        0
    }

    /// Returns the glyph index of the end of the line containing `cursor_i`.
    fn line_end_for(&self, cursor_i: usize) -> usize {
        for line in &self.layout.lines {
            if cursor_i <= line.end {
                return line.end;
            }
        }
        self.layout.glyphs.len()
    }

    /// Returns the line index for a given cursor position.
    fn cursor_line_idx(&self, cursor_i: usize) -> usize {
        for (i, line) in self.layout.lines.iter().enumerate() {
            if cursor_i <= line.end {
                return i;
            }
        }
        self.layout.lines.len().saturating_sub(1)
    }

    /// Deletes chars `[start, end)` from `self.text` and sets `dirty`.
    pub fn delete_range(&mut self, range: Range<usize>) {
        let byte_start = char_to_byte(&self.text, range.start);
        let byte_end = char_to_byte(&self.text, range.end);
        self.text.replace_range(byte_start..byte_end, "");
        self.dirty = true;
    }
}

impl Widget for TextInput {
    fn build_layout(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let mut offset = context.pos_child(FlexAlign::default(), Vec2::zero());
        let align_size = context.space();

        context.place(context.element_size);

        offset.y = self.align.get_y(align_size.y, self.layout.size.y, offset.y);

        context.apply_pos(offset);
        offset.y = offset.y.floor();

        for line in &self.layout.lines {
            let mut offset = offset;
            offset.x = self.align.get_x(align_size.x, line.width, offset.x);

            for c in &mut self.layout.glyphs[line.range()] {
                c.pos += offset;
            }
        }
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let size = Vec2::new(context.fill_size_x(1.0), self.layout.size.y);
        context.place(size);
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        self.dirty = false;

        context.fill_x(1.0);

        let text = if self.text.is_empty() {
            if self.placeholder.is_empty() {
                "\u{200B}"
            } else {
                self.placeholder
            }
        } else {
            &self.text
        };

        self.layout.build(text, context);
        context.predict(Vec2::new(0.0, self.layout.size.y));
    }

    fn draw_data(&mut self, element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {
        let font = self.layout.font(info.font);
        let color = if self.text.is_empty() {
            self.placeholder_color
        } else {
            self.color
        };
        let mat = font.material();
        let mut batch = resources.batch_data::<MSDFVertex>(mat, info);

        for glyph in &self.layout.glyphs {
            if glyph.size.x == 0.0 {
                continue;
            }

            let px_range = (glyph.size.y / (glyph.uv_end.y - glyph.uv_start.y)) * 4.0;

            let to_add = [
                MSDFVertex {
                    color,
                    pos: glyph.pos,
                    uv_pos: glyph.uv_start,
                    px_range,
                },
                MSDFVertex {
                    color,
                    pos: Vec2::new(glyph.pos.x + glyph.size.x, glyph.pos.y),
                    uv_pos: Vec2::new(glyph.uv_end.x, glyph.uv_start.y),
                    px_range,
                },
                MSDFVertex {
                    color,
                    pos: Vec2::new(glyph.pos.x, glyph.pos.y + glyph.size.y),
                    uv_pos: Vec2::new(glyph.uv_start.x, glyph.uv_end.y),
                    px_range,
                },
                MSDFVertex {
                    color,
                    pos: glyph.pos + glyph.size,
                    uv_pos: glyph.uv_end,
                    px_range,
                },
            ];

            batch.push_rect(&to_add);
        }

        let line_height = (self.layout.font_size * info.scale_factor * font.line_height) as i16;

        if let Some(selection) = &self.selection {
            let range = selection.range();

            if range.start != range.end {
                let glyphs = &self.layout.glyphs;
                let gl_len = glyphs.len();

                for (line_idx, line) in self.layout.lines.iter().enumerate() {
                    if line.end <= range.start || line.start >= range.end {
                        continue;
                    }

                    let x_start = if range.start <= line.start {
                        glyphs[line.start.min(gl_len.saturating_sub(1))].pos.x
                    } else {
                        glyphs[range.start].pos.x
                    } as i16;

                    // Extend to container's right edge when selection continues past this line
                    // (mirrors browser behavior where wrapped/continued lines fill to the edge)
                    let x_end = if range.end >= line.end && line.end < gl_len {
                        (element.pos.x as i16 + element.size.x) as f32
                    } else if range.end >= line.end {
                        let g = &glyphs[(line.end - 1).min(gl_len - 1)];
                        g.pos.x + g.size.x
                    } else {
                        glyphs[range.end].pos.x
                    }
                    .ceil() as i16;

                    let y = element.pos.y as i16 + line_idx as i16 * line_height;
                    resources.add(
                        MatType::Basic,
                        UiInstance {
                            color: hex_rgba!("#ff6b35a1"),
                            border_color: RGBA::ZERO,
                            border: [0; 4],
                            pos: Vec2::new(x_start, y),
                            size: Vec2::new((x_end - x_start).max(1), line_height),
                            corner: 0,
                        },
                        info,
                    );
                }
            }
        } else if let Some(cursor) = &self.cursor
            && cursor.is_on
        {
            let ci = cursor.index;
            let posx = if ci == 0 {
                if self.layout.glyphs.is_empty() {
                    return;
                }
                self.layout.glyphs[0].pos.x
            } else if let Some(g) = self.layout.glyphs.get(ci - 1) {
                g.pos.x + g.size.x
            } else {
                return;
            } as i16;

            let line_idx = self.cursor_line_idx(ci);
            let y = element.pos.y as i16 + line_idx as i16 * line_height;

            resources.add(
                MatType::Basic,
                UiInstance {
                    color: self.color,
                    border_color: RGBA::ZERO,
                    border: [0; 4],
                    pos: Vec2::new(posx, y),
                    size: Vec2::new((line_height / 12).max(1), line_height),
                    corner: 0,
                },
                info,
            );
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
                Self::unfocus(ui, element, StateChange::Submit);
                println!("unfocus submit");
                return InputResult::None;
            }
            UiEvent::Press if self.selectable => {
                ui.selection.set_capture(element);
                if self.focus_on_click && self.cursor.is_none() {
                    self.focus(ui, element);
                    self.set_new_cursor();
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

        if self.selectable {
            ui.cursor_icon = CursorIcon::Text;
        }

        InputResult::New
    }

    fn key_event(&mut self, element: UiRef, ui: &mut Ui, event: &KeyEvent) -> InputResult {
        if event.state != ElementState::Pressed {
            return InputResult::None;
        }

        if let Some(call) = self.on_input {
            let mut context = TextInputContext::new(ui, element, event);
            call(&mut context);

            if !matches!(context.submit, StateChange::None) {
                let reason = context.submit;
                Self::unfocus(ui, element, reason);
                ui.selection.focused = None;
                return InputResult::New;
            }

            if context.ingore {
                return InputResult::None;
            }
        }

        let shift = ui.modifiers.contains(KeyModifiers::Shift);
        let ctrl = ui.modifiers.contains(KeyModifiers::Ctrg);

        // Ctrl shortcuts
        if ctrl {
            if let Key::Character(c) = &event.logical_key {
                match c.as_str() {
                    "a" | "A" => {
                        let len = self.layout.glyphs.len();
                        if len > 0 {
                            self.selection = Some(Selection {
                                anchor: 0,
                                range: len,
                            });
                            self.cursor.as_mut().unwrap().index = len;
                            ui.color_changed();
                        }
                        return InputResult::New;
                    }
                    _ => {}
                }
            }
        }

        ui.set_event(QueuedEvent::new(&element, UiEvent::TextInput, self.message));

        if let Key::Named(name) = event.logical_key {
            match name {
                NamedKey::ArrowRight => {
                    if let Some(sel) = self.selection.take()
                        && !shift
                    {
                        let range = sel.range();
                        self.set_cursor(range.end);
                    } else if shift {
                        self.extend_selection(1);
                    } else {
                        self.move_cursor(1);
                    }
                    ui.color_changed();
                }
                NamedKey::ArrowLeft => {
                    if let Some(sel) = self.selection.take()
                        && !shift
                    {
                        let range = sel.range();
                        self.set_cursor(range.start);
                    } else if shift {
                        self.extend_selection(-1);
                    } else {
                        self.move_cursor(-1);
                    }
                    ui.color_changed();
                }
                NamedKey::Home => {
                    let cursor_i = self.cursor.as_ref().unwrap().index;
                    let target = self.line_start_for(cursor_i);
                    if shift {
                        self.extend_selection_to(target);
                    } else {
                        self.selection = None;
                        self.set_cursor(target);
                    }
                    ui.color_changed();
                    return InputResult::New;
                }
                NamedKey::End => {
                    let cursor_i = self.cursor.as_ref().unwrap().index;
                    let target = self.line_end_for(cursor_i);
                    if shift {
                        self.extend_selection_to(target);
                    } else {
                        self.selection = None;
                        self.set_cursor(target);
                    }
                    ui.color_changed();
                    return InputResult::New;
                }
                NamedKey::Backspace => {
                    if let Some(sel) = self.selection.take() {
                        let range = sel.range();
                        if range.start != range.end {
                            self.delete_range(range);
                            self.set_cursor(range.start);
                            ui.layout_changed();
                            return InputResult::New;
                        }
                    }
                    let cursor_pos = self.cursor.as_ref().unwrap().index;
                    if cursor_pos != 0 {
                        self.delete_range(Range {
                            start: cursor_pos - 1,
                            end: cursor_pos,
                        });
                        self.move_cursor(-1);
                        ui.layout_changed();
                        return InputResult::New;
                    }
                    return InputResult::None;
                }
                NamedKey::Delete => {
                    if let Some(sel) = self.selection.take() {
                        let range = sel.range();
                        if range.start != range.end {
                            self.delete_range(range);
                            self.set_cursor(range.start);
                            ui.layout_changed();
                            return InputResult::New;
                        }
                    }
                    let cursor_pos = self.cursor.as_ref().unwrap().index;
                    let text_len = self.text.len();
                    if text_len != 0 && cursor_pos < text_len {
                        self.delete_range(Range {
                            start: cursor_pos,
                            end: cursor_pos + 1,
                        });
                        ui.layout_changed();
                        return InputResult::New;
                    }
                    return InputResult::None;
                }
                _ => (),
            }
        }

        if let Some(text) = &event.text
            && !text.is_empty()
            && !ctrl
        {
            // Replace selection if active
            if let Some(sel) = self.selection.take() {
                let range = sel.range();
                self.delete_range(range);
                self.cursor.as_mut().unwrap().index = range.start;
            }
            let cursor_pos = self.cursor.as_ref().unwrap().index;
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
            placeholder: "text eingeben",
            placeholder_color: RGBA::grey(150),

            text: String::new(),
            color: RGBA::WHITE,
            layout: TextLayout::default(),
            align: Align::Left,

            selectable: true,
            focus_on_click: true,
            cursor: None,
            selection: None,

            message: 0,
            on_input: Some(default_on_input),
            state_change: None,

            dirty: true,
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
        ctx.submit = StateChange::Submit;
    } else if ctx.event.logical_key == Key::Named(NamedKey::Escape) {
        ctx.submit = StateChange::Escape
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

    pub fn range(&self) -> Range<usize> {
        if self.anchor <= self.range {
            Range {
                start: self.anchor,
                end: self.range,
            }
        } else {
            Range {
                start: self.range,
                end: self.anchor,
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StateChange {
    Focus,
    Submit,
    ClickOutside,
    Escape,
    None,
}
