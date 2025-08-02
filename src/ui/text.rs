use super::{
    Align, BuildContext, ElementBuild, ElementType, UiElement, UiState,
    ui_element::{Element, TypeConst},
};
use crate::{
    graphics::{FontInstance, UiInstance, formats::Color},
    primitives::Vec2,
};

pub struct Text {
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub align: Align,
    pub wrap: WrapMode,
    pub line_spacing: f32,
    pub dirty_flags: TextDirtyFlags,
    pub font_instances: Vec<FontInstance>,
}

impl Text {
    pub fn get_font_instances(&mut self, parent_size: Vec2, parent_pos: Vec2, ui: &mut UiState) {
        match self.dirty_flags {
            TextDirtyFlags::None => ui.texts.extend_from_slice(&self.font_instances),
            TextDirtyFlags::TextChanged => {
                let mut context = BuildContext::default(&ui.font, parent_size);
                context.parent_pos = parent_pos;
                self.build(&mut context);
                ui.texts.extend_from_slice(&self.font_instances)
            }
            TextDirtyFlags::AddedChar => todo!(),
            TextDirtyFlags::RemovedChar => todo!(),
        }
    }

    pub fn set_new(&mut self, text: &str) {
        self.text = text.to_string();
        self.dirty_flags = TextDirtyFlags::TextChanged;
    }
}

impl Element for Text {
    fn build(&mut self, context: &mut BuildContext) {
        self.dirty_flags = TextDirtyFlags::None;
        self.font_instances.clear();
        let font = context.font();
        let font_uv_height = 8;
        let scale_factor = self.font_size / font_uv_height as f32;
        let mut cursor_pos = Vec2::zero();

        for c in self.text.chars() {
            if c == ' ' {
                cursor_pos.x += self.font_size * 0.5;
            } else if c == '\n' {
                cursor_pos.x = 0.0;
                cursor_pos.y += self.font_size + self.line_spacing;
            } else {
                let char_data = font.get_data(c as u8);
                let uv_start = (char_data.0, char_data.1);
                let uv_size = (char_data.2, font_uv_height);

                let font_instance = FontInstance {
                    color: self.color,
                    pos: cursor_pos,
                    size: Vec2::new(
                        char_data.2 as f32 * scale_factor,
                        font_uv_height as f32 * scale_factor,
                    ),
                    uv_start,
                    uv_size,
                };

                self.font_instances.push(font_instance);
                cursor_pos.x += char_data.2 as f32 * scale_factor;
            }
        }

        let mut offset = context.parent_pos;
        if matches!(self.align, Align::Center) {
            offset.x += (context.parent_size.x - cursor_pos.x) * 0.5;
            offset.y += (context.parent_size.y - self.font_size) * 0.5;
        }

        for i in &mut self.font_instances {
            i.pos += offset
        }
    }

    fn instance(&self) -> UiInstance {
        UiInstance::default()
    }

    fn childs(&mut self) -> &mut [UiElement] {
        &mut []
    }

    fn add_child(&mut self, _child: UiElement) {}
}

impl ElementBuild for Text {
    fn wrap(self, ui_state: &super::UiState) -> UiElement {
        UiElement {
            id: ui_state.get_id(),
            typ: ElementType::Text,
            dirty: true,
            visible: true,
            size: Vec2::new(0.0, 0.0),
            pos: Vec2::new(0.0, 0.0),
            parent: std::ptr::null_mut(),
            element: Box::new(self),
        }
    }
}

impl TypeConst for Text {
    const ELEMENT_TYPE: ElementType = ElementType::Text;
}

impl Default for Text {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            text: "Default".to_string(),
            font_size: 16.0,
            font_instances: Vec::new(),
            align: Align::Top,
            line_spacing: 8.0,
            wrap: WrapMode::default(),
            dirty_flags: TextDirtyFlags::TextChanged,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WrapMode {
    Character,
    Word,
    None,
}

impl Default for WrapMode {
    fn default() -> Self {
        WrapMode::Word
    }
}

pub enum TextDirtyFlags {
    None,
    TextChanged,
    AddedChar,
    RemovedChar,
}
