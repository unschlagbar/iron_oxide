use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, TextInput, UiElement, UiRef,
        materials::{FontInstance, MatType, UiInstance},
        text_input::InputCursor,
        text_layout::{LayoutText, TextLayout},
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
    pub build_layout: LayoutText,
    pub draw_data: Vec<FontInstance>,
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
            build_layout: LayoutText::default(),
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

        let align = self.align;
        let mut offset = context.pos_child(FlexAlign::default(), Vec2::zero());
        let align_size = context.size();
        let font_size = self.layout.font_size * context.scale_factor;

        context.place_child(context.element_size);

        if align.vertical_centered() {
            offset.y +=
                (align_size.y - font_size * self.build_layout.lines.len() as f32).max(0.0) * 0.5;
        }

        for line in &self.build_layout.lines {
            let mut offset = offset;
            if align.horizontal_centered() {
                offset.x += (align_size.x - line.width) * 0.5;
            }

            for c in &line.content {
                self.draw_data.push(FontInstance {
                    color: self.color,
                    pos: offset + c.pos,
                    size: c.size,
                    uv_start: c.uv_start,
                    uv_size: c.uv_size,
                });
            }
        }
        context.apply_pos(offset);
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        context.place_child(self.build_layout.size);
        context.apply_size(self.build_layout.size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        self.dirty = false;

        let text = if self.text.is_empty() {
            "\u{200B}"
        } else {
            &self.text
        };

        self.build_layout = self.layout.build(text, context);
        context.predict_child(self.build_layout.size);
    }

    fn draw_data(&mut self, _element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {
        ressources.add_slice(MatType::Font, &self.draw_data, info);

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
            build_layout: LayoutText::default(),
            draw_data: Vec::new(),
        }
    }
}
