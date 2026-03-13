use std::slice;

use crate::{
    graphics::{Resources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, TextInput, UiElement, UiRef, materials::MSDFInstance,
        text_layout::TextLayout, units::FlexAlign, widget::Widget,
    },
};

pub struct Text {
    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub selectable: bool,

    pub dirty: bool,
}

impl Text {
    pub fn from(text_input: TextInput) -> Self {
        Self {
            text: text_input.text,
            color: text_input.color,
            layout: text_input.layout,
            align: text_input.align,
            selectable: text_input.selectable,
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
}

impl Widget for Text {
    fn build_layout(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let mut offset = context.pos_child(FlexAlign::default(), Vec2::zero());
        let align_size = context.size();

        context.place_child(context.element_size);

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

    fn draw_data(&mut self, _element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {
        let font = self.layout.font(info.font);
        let mat = font.material();

        let batch = resources.batch_data::<MSDFInstance>(mat, info);
        batch.reserve(self.layout.glyphs.len() * size_of::<MSDFInstance>());

        for glyph in &self.layout.glyphs {
            if glyph.size.x == 0.0 {
                continue;
            }

            let to_add = MSDFInstance {
                color: self.color,
                pos: glyph.pos,
                size: glyph.size,
                uv_start: glyph.uv_start,
                uv_end: glyph.uv_end,
            };
            let slice = unsafe {
                slice::from_raw_parts(
                    &to_add as *const MSDFInstance as *const u8,
                    size_of_val(&to_add),
                )
            };
            batch.extend_from_slice(slice);
        }
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

            dirty: true,
        }
    }
}
