use ash::vk::Rect2D;

use super::{
    BuildContext, ElementType, UiElement, UiState,
    element::{Element, TypeConst},
};
use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{
        Align,
        materials::FontInstance,
        text_layout::{TextDirtyFlags, TextLayout},
    },
};

pub struct Text {
    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub dirty_flags: TextDirtyFlags,
    pub font_instances: Vec<FontInstance>,
}

impl Text {
    pub fn get_font_instances(
        &mut self,
        parent_size: Vec2,
        parent_pos: Vec2,
        ui: &mut UiState,
        element: &UiElement,
        clip: Option<Rect2D>,
    ) {
        match self.dirty_flags {
            TextDirtyFlags::None => {
                for inst in &self.font_instances {
                    ui.materials[1].add(inst as *const _ as *const _, 0, clip)
                }
            }
            TextDirtyFlags::TextChanged => {
                let mut context = BuildContext::default(&ui.font, parent_size);
                context.child_start_pos = parent_pos;
                self.build(&mut context, element);
                for inst in &self.font_instances {
                    ui.materials[1].add(inst as *const _ as *const _, 0, clip)
                }
            }
            TextDirtyFlags::AddedChar => todo!(),
            TextDirtyFlags::RemovedChar => todo!(),
        }
    }

    pub fn set_new(&mut self, text: String) {
        self.text = text;
        self.dirty_flags = TextDirtyFlags::TextChanged;
    }
}

impl Element for Text {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement) {
        self.dirty_flags = TextDirtyFlags::None;
        self.font_instances.clear();

        let align = self.align;
        let mut offset = context.child_start_pos;
        offset.y += context.used_main;

        let font_size = self.layout.font_size;
        let layout = self.layout.build(&self.text, context);

        if align.vertical_centered() {
            offset.y +=
                (context.available_size.y - font_size * layout.lines.len() as f32).max(0.0) * 0.5;
        }

        for line in &layout.lines {
            let mut offset = offset;
            if align.horizontal_centered() {
                offset.x += (context.available_size.x - line.width) * 0.5;
            }

            for c in &line.content {
                self.font_instances.push(FontInstance {
                    color: self.color,
                    pos: offset + c.pos,
                    size: c.size,
                    uv_start: c.uv_start,
                    uv_size: c.uv_size,
                    z_index: element.z_index,
                });
            }
        }

        context.place_child(layout.size);
        context.apply_data(offset, layout.size);

        println!("{}, {}", offset, layout.size);
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
            dirty_flags: TextDirtyFlags::TextChanged,
            font_instances: Vec::new(),
        }
    }
}
