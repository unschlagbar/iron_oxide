use ash::vk::Rect2D;

use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, TextInput, UiElement, UiRef,
        materials::{FontInstance, MatType, UiInstance},
        text_input::InputCursor,
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

    pub dirty: bool,
    pub font_instances: Vec<FontInstance>,
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
            font_instances: text_input.font_instances,
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

        let font_size = self.layout.font_size * context.scale_factor;

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
        _: f32,
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

            dirty: true,
            font_instances: Vec::new(),
        }
    }
}
