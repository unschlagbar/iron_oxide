use std::{ops::Range, time::Instant};

use ash::vk::Rect2D;

use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
    ui::{
        Align, BuildContext, UiElement, UiEvent, UiRef, UiState, element::Element, materials::{FontInstance, UiInstance}, text_layout::TextLayout
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

    pub fn focus(ui: &mut UiState, element: &UiElement, _select: Range<usize>) {
        ui.set_focus(element);
        let this: &mut Self = UiRef::new_ref(element).get_mut(ui).downcast_mut().unwrap();

        let last_char = this.font_instances.last().unwrap();
        let pos = last_char.pos + Vec2::new(last_char.size.x, 0.0);

        this.cursor = Some(InputCursor { pos, start_time: Instant::now(), is_on: true });

        ui.set_ticking(element);
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

    fn instance(&mut self, element: &UiElement, ui: &mut UiState, clip: Option<Rect2D>) -> Option<Rect2D> {
        if self.dirty {
            let parent = unsafe { element.parent.unwrap().as_ref() };
            let mut context = BuildContext::default(&ui.font, parent.size);
            context.child_start_pos = parent.pos;
            self.build(element.childs_mut(), &mut context);
        }

        for inst in &self.font_instances {
            ui.materials[1].add(inst.to_add(), 0, clip)
        }

        if let Some(cursor) = &self.cursor && cursor.is_on {
            let scale = self.layout.font_size * 1.2 - self.layout.font_size;
            let material = &mut ui.materials[0];
            let to_add = UiInstance {
                color: self.color,
                border_color: RGBA::ZERO,
                border: [0; 4],
                x: cursor.pos.x as _,
                y: (cursor.pos.y - scale * 0.5) as _,
                width: 2,
                height: (self.layout.font_size + scale) as _,
                corner: 0.0,
                z_index: element.z_index,
            };
            material.add(to_add.to_add(), 0, clip);
        }

        clip
    }

    fn is_ticking(&self) -> bool {
        self.cursor.is_some()
    }

    fn tick(&mut self, _element: super::UiRef, ui: &mut UiState) {
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

    fn has_interaction(&self) -> bool {
        true
    }

    fn interaction(&mut self, _element: UiRef, _ui: &mut UiState, event: super::UiEvent) -> super::EventResult {
        if event == UiEvent::End && self.cursor.is_some() {
            println!("fire done");

            self.cursor = None;
            super::EventResult::New
        } else {
            super::EventResult::None
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
            cursor: None,

            dirty: true,
            font_instances: Vec::new(),
        }
    }
}

pub struct InputCursor {
    pos: Vec2,
    start_time: Instant,
    is_on: bool,
}

impl InputCursor {
    fn _pos_from_text(&mut self, idx: usize, text: &[FontInstance]) {
        let char = &text[idx];
        self.pos = char.pos
    }
}
