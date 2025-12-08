use ash::vk::Rect2D;

use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{
        Align, BuildContext, ElementType, Text, TypeConst, UiState,
        element::Element,
        materials::FontInstance,
        text_layout::{TextDirtyFlags, TextLayout},
    },
};

pub struct TextInput {
    pub text: String,
    pub color: RGBA,
    pub layout: TextLayout,
    pub align: Align,

    pub selectable: bool,
    pub cursor: Option<(u32, u32)>,

    pub dirty_flags: TextDirtyFlags,
    pub font_instances: Vec<FontInstance>,
}

impl TextInput {
    pub fn get_font_instances(
        &mut self,
        parent_size: Vec2,
        parent_pos: Vec2,
        ui: &mut UiState,
        clip: Option<Rect2D>,
    ) {
        match self.dirty_flags {
            TextDirtyFlags::None => {
                for inst in &self.font_instances {
                    ui.materials[1].add(inst, 0, clip)
                }
            }
            TextDirtyFlags::TextChanged => {
                let mut context = BuildContext::default(&ui.font, parent_size);
                context.child_start_pos = parent_pos;
                self.build(&mut context);
                for inst in &self.font_instances {
                    ui.materials[1].add(inst, 0, clip)
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

    pub fn handle_input(&mut self, input: &str) {
        println!("Text {}", input);
    }

    pub fn to_text(self) -> Text {
        Text {
            text: self.text,
            color: self.color,
            layout: self.layout,
            align: self.align,
            selectable: self.selectable,
            dirty_flags: self.dirty_flags,
            font_instances: self.font_instances,
        }
    }
}

impl Element for TextInput {
    fn build(&mut self, context: &mut BuildContext) {
        self.dirty_flags = TextDirtyFlags::None;
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
}

impl TypeConst for TextInput {
    const ELEMENT_TYPE: ElementType = ElementType::Text;
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            text: "Text".to_string(),
            color: RGBA::WHITE,
            layout: TextLayout::default(),
            align: Align::default(),

            selectable: true,
            cursor: None,

            dirty_flags: TextDirtyFlags::TextChanged,
            font_instances: Vec::new(),
        }
    }
}
