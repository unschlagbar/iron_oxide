use crate::{
    graphics::{Resources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, DrawInfo, TextInput, UiElement, UiRect, UiRef, materials::MSDFVertex,
        text_layout::TextLayout, units::FlexAlign, widget::Widget,
    },
};

pub struct Text {
    pub text: String,
    pub color: RGBA,
    /// Colour changes inside the run, as `(first character, colour)` sorted by
    /// position — everything from one entry to the next is drawn in its colour,
    /// and anything before the first entry in `color`. This is what lets one
    /// wrapped paragraph say different things about different words instead of
    /// being cut into a stack of one-colour boxes.
    pub runs: Vec<(usize, RGBA)>,
    pub layout: TextLayout,
    pub align: Align,
    pub margin: UiRect,

    pub selectable: bool,

    pub dirty: bool,
}

impl Text {
    pub fn from(text_input: TextInput) -> Self {
        Self {
            text: text_input.text,
            color: text_input.color,
            runs: Vec::new(),
            layout: text_input.layout,
            align: text_input.align,
            margin: UiRect::default(),
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
        let mut offset =
            context.pos_child(FlexAlign::default(), Vec2::zero()) + self.margin.start(context);
        let align_size = context.space();

        context.place(context.element_size);

        offset.y = self.align.get_y(align_size.y, self.layout.size.y, offset.y);

        context.apply_pos(offset);
        offset.y = offset.y.floor();

        self.layout.place(self.align, align_size, offset);
    }

    fn build_size(&mut self, _: &mut [UiElement], ctx: &mut BuildContext) {
        let size = self.layout.size + self.margin.size(ctx);
        ctx.place(size);
        ctx.apply_size(size);
    }

    fn predict_size(&mut self, ctx: &mut BuildContext) {
        let text = if self.text.is_empty() {
            "\u{200B}"
        } else {
            &self.text
        };

        // Nothing about this run has changed in most passes, and laying it out
        // again would produce the glyphs already standing.
        if self.dirty || !self.layout.is_current(text, ctx) {
            self.layout.build(text, ctx);
        }
        self.dirty = false;

        ctx.predict(self.layout.size + self.margin.size(ctx));
    }

    fn draw_data(&mut self, _element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {
        let font = self.layout.font(info.font);
        let mat = font.material();

        let mut batch = resources.batch_data::<MSDFVertex>(mat, info);

        // Walked in step with the glyphs rather than searched per glyph: both
        // are in character order, so the colour of the next glyph is never
        // further than one entry away.
        let mut run = 0;
        let mut color = self.color;

        for glyph in &self.layout.glyphs {
            while run < self.runs.len() && self.runs[run].0 <= glyph.index as usize {
                color = self.runs[run].1;
                run += 1;
            }

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
                    pos: Vec2::new(glyph.pos.x + glyph.size.x, glyph.pos.y + glyph.size.y),
                    uv_pos: glyph.uv_end,
                    px_range,
                },
            ];

            batch.push_rect(&to_add);
        }
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            text: "Text".to_string(),
            color: RGBA::WHITE,
            runs: Vec::new(),
            layout: TextLayout::default(),
            align: Align::default(),
            margin: UiRect::default(),

            selectable: true,

            dirty: true,
        }
    }
}
