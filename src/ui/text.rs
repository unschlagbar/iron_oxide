use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, TextInput, UiElement, UiRef,
        materials::{AtlasInstance, MatType, UiInstance},
        text_input::InputCursor,
        text_layout::TextLayout,
        units::FlexAlign,
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

    pub dirty: bool,
    pub draw_data: Vec<AtlasInstance>,
}

impl Text {
    pub fn from(text_input: TextInput) -> Self {
        Self {
            text: text_input.text,
            color: text_input.color,
            layout: text_input.layout,
            align: text_input.align,
            selectable: text_input.selectable,
            cursor: text_input.cursor,
            dirty: false,
            draw_data: text_input.draw_data,
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
}

impl Widget for Text {
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
                    uv_start: c.uv_start,
                    uv_size: c.uv_size,
                });
            }
        }
    }

    fn build_size(&mut self, _: &mut [UiElement], ctx: &mut BuildContext) {
        ctx.place_child(self.layout.size);
        ctx.apply_size(self.layout.size);
    }

    fn predict_size(&mut self, ctx: &mut BuildContext) {
        self.dirty = false;

        let text = if self.text.is_empty() {
            "\u{200B}"
        } else {
            &self.text
        };

        self.layout.build(text, ctx);
        ctx.predict_child(self.layout.size);
    }

    fn draw_data(&mut self, _element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {
        let mat = if let Some(font) = &self.layout.font {
            if font.bitmap {
                MatType::Bitmap
            } else {
                MatType::MSDF
            }
        } else {
            MatType::MSDF
        };
        ressources.add_slice(mat, &self.draw_data, info);

        if let Some(cursor) = &self.cursor
            && cursor.is_on
        {
            let pos = if cursor.index == 0 {
                self.draw_data[0].pos
            } else if let Some(char) = self.draw_data.get(cursor.index - 1) {
                char.pos + Vec2::new(char.size.x, 0.0)
            } else {
                return;
            };

            let scale = self.layout.font_size * 1.2 - self.layout.font_size;
            let to_add = UiInstance {
                color: self.color,
                border_color: RGBA::ZERO,
                border: [0; 4],
                pos: Vec2::new(pos.x as i16, (pos.y - scale * 0.5) as i16),
                size: Vec2::new(
                    2 * info.scale_factor as i16,
                    (self.layout.font_size * info.scale_factor + scale) as i16,
                ),
                corner: 0,
            };
            ressources.add(MatType::Basic, to_add, info);
        }
    }

    fn is_ticking(&self) -> bool {
        self.cursor.is_some()
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            text: "Text".to_string(),
            color: RGBA::grey(220),
            layout: TextLayout::default(),
            align: Align::default(),

            selectable: true,
            cursor: None,

            dirty: true,
            draw_data: Vec::new(),
        }
    }
}
