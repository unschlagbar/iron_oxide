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
        text_layout::{TextLayout, TextLine},
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
    /// Enter writes a line break instead of submitting. A field the user is
    /// meant to compose a paragraph in, rather than one line they send.
    pub multiline: bool,
    /// Losing the focus leaves the selection standing. A control outside the
    /// field can then act on what is selected in it — which it could not if the
    /// click that reached the control had already cleared it.
    pub keep_selection: bool,
    pub cursor: Option<InputCursor>,
    pub selection: Option<Selection>,
    /// Ranges tinted behind the text, drawn like a selection but owned by the
    /// caller: what the field says *about* its content, not what the pointer is
    /// doing to it.
    pub highlights: Vec<Highlight>,

    pub message: u16,
    pub on_input: Option<fn(&mut TextInputContext)>,
    pub state_change: Option<fn(StateChangeCtx)>,

    pub dirty: bool,
}

/// Every index this widget takes or hands out — the cursor, a selection, a
/// highlight — counts **characters of `text`**. Glyphs are not the same count:
/// a line break and a collapsed space lay out none, so a glyph index is one
/// character short per break before it. Glyph indices live inside drawing and
/// hit testing only, and [`TextInput::char_of_glyph`] /
/// [`TextInput::glyph_of_char`] are the only way across.
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
            multiline: false,
            keep_selection: false,
            cursor: None,
            selection: None,
            highlights: Vec::new(),

            message: 0,
            on_input: Some(default_on_input),
            state_change: None,
            dirty: false,
        }
    }
    /// Paints `range` line by line, the way a selection is painted: a run that
    /// continues past the end of a line fills to the field's right edge, so a
    /// wrapped range reads as one block rather than as ragged pieces.
    fn fill_range(
        &self,
        range: Range<usize>,
        color: RGBA,
        element: UiRef,
        line_height: i16,
        resources: &mut Resources,
        info: &mut DrawInfo,
    ) {
        let glyphs = &self.layout.glyphs;
        let gl_len = glyphs.len();
        if range.start >= range.end || gl_len == 0 {
            return;
        }

        for (line_idx, line) in self.layout.lines.iter().enumerate() {
            let line_start = self.char_of_glyph(line.start);
            let line_end = self.line_char_end(line);
            if line_end <= range.start || line_start >= range.end {
                continue;
            }

            let from = self.glyph_of_char(range.start.max(line_start));
            let x_start = glyphs[from.min(gl_len - 1)].pos.x as i16;

            // A run that carries on past this line fills to the field's right
            // edge, so a wrapped range reads as one block and not as pieces.
            let x_end = if range.end >= line_end && line.end < gl_len {
                (element.pos.x as i16 + element.size.x) as f32
            } else if range.end >= line_end {
                let g = &glyphs[(line.end - 1).min(gl_len - 1)];
                g.pos.x + g.size.x
            } else {
                glyphs[self.glyph_of_char(range.end).min(gl_len - 1)].pos.x
            }
            .ceil() as i16;

            let y = element.pos.y as i16 + line_idx as i16 * line_height;
            resources.add(
                MatType::Basic,
                UiInstance {
                    color,
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

    /// The character a glyph was laid out from; the end of the text for a
    /// glyph index past the last one.
    fn char_of_glyph(&self, glyph: usize) -> usize {
        match self.layout.glyphs.get(glyph) {
            Some(glyph) => glyph.index as usize,
            None => self.char_len(),
        }
    }

    /// The first glyph at or after character `at`. The layout is built in
    /// character order, so this is a binary search rather than a scan.
    fn glyph_of_char(&self, at: usize) -> usize {
        self.layout
            .glyphs
            .partition_point(|g| (g.index as usize) < at)
    }

    /// One past the last character of `line` — where a click past the end of
    /// that line lands, which is the break itself rather than the first
    /// character of the next line.
    fn line_char_end(&self, line: &TextLine) -> usize {
        match line.end.checked_sub(1).and_then(|i| self.layout.glyphs.get(i)) {
            Some(glyph) => glyph.index as usize + 1,
            None => self.char_of_glyph(line.start),
        }
    }

    fn char_len(&self) -> usize {
        self.text.chars().count()
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
        let index = self.char_len();
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
        if !this.keep_selection {
            this.selection = None;
        }

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
        let char_len = self.char_len();

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

    /// Returns the character index closest to `cursor_pos` in screen space.
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
            return self.char_len();
        }
        let g = &glyphs[line.start];

        // Snap to line boundaries when cursor is in a vertical gap
        if cursor_pos.y > g.pos.y + g.size.y {
            return self.line_char_end(line);
        }
        if cursor_pos.y < g.pos.y {
            return self.char_of_glyph(line.start);
        }

        for i in line.start..line_end {
            if cursor_pos.x < glyphs[i].pos.x + glyphs[i].size.x * 0.5 {
                return self.char_of_glyph(i);
            }
        }
        self.line_char_end(line)
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

    /// The first character of the line the cursor is on.
    fn line_start_for(&self, cursor_i: usize) -> usize {
        for line in &self.layout.lines {
            if cursor_i <= self.line_char_end(line) {
                return self.char_of_glyph(line.start);
            }
        }
        0
    }

    /// The last character of the line the cursor is on, the break excluded.
    fn line_end_for(&self, cursor_i: usize) -> usize {
        for line in &self.layout.lines {
            if cursor_i <= self.line_char_end(line) {
                return self.line_char_end(line);
            }
        }
        self.char_len()
    }

    /// Which line the cursor is on.
    fn cursor_line_idx(&self, cursor_i: usize) -> usize {
        for (i, line) in self.layout.lines.iter().enumerate() {
            if cursor_i <= self.line_char_end(line) {
                return i;
            }
        }
        self.layout.lines.len().saturating_sub(1)
    }

    /// Writes `text` where the cursor is, over the selection if there is one.
    /// One typed character and a whole pasted paragraph are the same edit.
    fn insert(&mut self, ui: &mut Ui, text: &str) {
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
    }

    /// What is selected, or nothing when the selection is empty — an anchor
    /// the user set and then never dragged off is a cursor, not a selection.
    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection.as_ref()?.range();
        (range.end > range.start).then(|| {
            self.text
                .chars()
                .skip(range.start)
                .take(range.end - range.start)
                .collect()
        })
    }

    /// Puts the selection on the clipboard. Nothing selected is nothing
    /// copied — a field does not fall back to handing over the whole line.
    #[cfg(feature = "clipboard")]
    fn copy(&self) {
        if let Some(text) = self.selected_text() {
            crate::clipboard::set_text(text);
        }
    }

    #[cfg(feature = "clipboard")]
    fn cut(&mut self, ui: &mut Ui, element: UiRef) {
        if let Some(text) = self.selected_text() {
            crate::clipboard::set_text(text);
            // With a selection standing, this deletes it and writes nothing in
            // its place.
            self.insert(ui, "");
            ui.set_event(QueuedEvent::new(&element, UiEvent::TextInput, self.message));
        }
    }

    #[cfg(feature = "clipboard")]
    fn paste(&mut self, ui: &mut Ui, element: UiRef) {
        if let Some(text) = crate::clipboard::text()
            && !text.is_empty()
        {
            self.insert(ui, &text);
            ui.set_event(QueuedEvent::new(&element, UiEvent::TextInput, self.message));
        }
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

        self.layout.place(self.align, align_size, offset);
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let size = Vec2::new(context.fill_size_x(1.0), self.layout.size.y);
        context.place(size);
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
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

        // A layout pass asks every element in the tree, and one keystroke marks
        // exactly one of them dirty; the rest already hold the glyphs the
        // rebuild would produce.
        if self.dirty || !self.layout.is_current(text, context) {
            self.layout.build(text, context);
        }
        self.dirty = false;

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

        // Behind the pointer's own selection: a highlight says something about
        // the text that outlives the click, so the selection has to stay
        // readable on top of it.
        for hl in &self.highlights {
            self.fill_range(hl.range.clone(), hl.color, element, line_height, resources, info);
        }

        if let Some(selection) = &self.selection {
            let range = selection.range();

            if range.start != range.end {
                self.fill_range(
                    range,
                    hex_rgba!("#ff6b35a1"),
                    element,
                    line_height,
                    resources,
                    info,
                );
            }
        } else if let Some(cursor) = &self.cursor
            && cursor.is_on
        {
            let ci = cursor.index;
            // The caret sits after the glyph before it, which is the glyph the
            // cursor's character maps back to.
            let gi = self.glyph_of_char(ci);
            let posx = if gi == 0 {
                if self.layout.glyphs.is_empty() {
                    return;
                }
                self.layout.glyphs[0].pos.x
            } else if let Some(g) = self.layout.glyphs.get(gi - 1) {
                // A glyph with no ink, a space above all, still moved the pen.
                if g.size.x == 0.0 {
                    g.pos.x
                        + font.get_glyph(g.char).advance * self.layout.font_size * info.scale_factor
                } else {
                    g.pos.x + g.size.x
                }
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
            // Losing the focus is not submitting: a click elsewhere, or another
            // input taking over, must not read as the user having pressed enter.
            UiEvent::End if self.cursor.is_some() => {
                Self::unfocus(ui, element, StateChange::ClickOutside);
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
            // A field has no scroll of its own — it is laid out at the height of
            // its text — so the wheel belongs to whatever is behind it. Claiming
            // it here stops the panel a field sits in from scrolling whenever
            // the pointer happens to rest on one.
            UiEvent::Scroll(_) => return InputResult::None,
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

        if self.multiline
            && event.logical_key == Key::Named(NamedKey::Enter)
            && !ui.modifiers.contains(KeyModifiers::Ctrg)
        {
            self.insert(ui, "\n");
            return InputResult::New;
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
                        let len = self.char_len();
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
                    #[cfg(feature = "clipboard")]
                    "c" | "C" => {
                        self.copy();
                        return InputResult::New;
                    }
                    #[cfg(feature = "clipboard")]
                    "x" | "X" => {
                        self.cut(ui, element);
                        return InputResult::New;
                    }
                    #[cfg(feature = "clipboard")]
                    "v" | "V" => {
                        self.paste(ui, element);
                        return InputResult::New;
                    }
                    _ => {}
                }
            }
        }

        // The dedicated editing keys, on a keyboard that has them. They are a
        // key of their own rather than ctrl and a letter, so nothing above sees
        // them — and nothing below should insert a character for them either.
        #[cfg(feature = "clipboard")]
        if let Key::Named(name) = event.logical_key {
            match name {
                NamedKey::Copy => {
                    self.copy();
                    return InputResult::New;
                }
                NamedKey::Cut => {
                    self.cut(ui, element);
                    return InputResult::New;
                }
                NamedKey::Paste => {
                    self.paste(ui, element);
                    return InputResult::New;
                }
                _ => (),
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
            self.insert(ui, text);
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
            multiline: false,
            keep_selection: false,
            cursor: None,
            selection: None,
            highlights: Vec::new(),

            message: 0,
            on_input: Some(default_on_input),
            state_change: None,

            dirty: true,
        }
    }
}

/// A range of the field's text tinted behind the glyphs. Indices are char
/// positions, the same unit the cursor and the selection count in.
#[derive(Clone, Debug)]
pub struct Highlight {
    pub range: Range<usize>,
    pub color: RGBA,
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
