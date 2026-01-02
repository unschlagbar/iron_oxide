use std::{ops::Range, time::Instant};

use ash::vk::Rect2D;

use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, ElementType, TypeConst, UiElement, UiState, element::Element,
        materials::FontInstance, text_layout::TextLayout,
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
    pub fn set_new(&mut self, text: String) {
        self.text = text;
        self.dirty = true;
    }

    pub fn push_text(&mut self, text: &str) {
        self.text += text;
        self.dirty = true;
    }

    pub fn handle_input(&mut self, input: &str) {
        println!("Text {}", input);
    }

    pub fn focus(_ui: &mut UiState, _element: &UiElement, _select: Range<usize>) {
        //let gg = element.do
    }
}

impl Element for Text {
    fn build(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        self.dirty = false;
        self.font_instances.clear();

        let align = self.align;
        let mut offset = context.pos_child();

        let font_size = self.layout.font_size;
        let layout = self.layout.build(&self.text, context);

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

    fn instance(&mut self, element: &UiElement, ui: &mut UiState, clip: Option<Rect2D>) {
        if self.dirty {
            let parent = unsafe { element.parent.unwrap().as_ref() };
            let mut context = BuildContext::default(&ui.font, parent.size);
            context.child_start_pos = parent.pos;
            self.build(element.childs_mut(), &mut context);
        }

        for inst in &self.font_instances {
            ui.materials[1].add(inst.to_add(), 0, clip)
        }
    }

    fn is_ticking(&self) -> bool {
        self.cursor.is_some()
    }

    fn tick(&mut self, _element: super::UiRef, ui: &mut UiState) {
        if let Some(cursor) = &mut self.cursor {
            if cursor.start_time.elapsed().as_secs() % 2 == 0 && !cursor.is_on {
                cursor.is_on = true;
                ui.color_changed();
            } else if cursor.is_on {
                cursor.is_on = false;
                ui.color_changed();
            }
        }
    }

    fn has_interaction(&self) -> bool {
        true
    }
}

impl TypeConst for Text {
    const ELEMENT_TYPE: ElementType = ElementType::Text;
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

pub struct InputCursor {
    _pos: Vec2,
    start_time: Instant,
    is_on: bool,
}

impl InputCursor {
    fn _pos_from_text(&mut self, idx: usize, text: &[FontInstance]) {
        let char = &text[idx];
        self._pos = char.pos
    }
}
