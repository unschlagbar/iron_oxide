use ash::vk::Rect2D;

use crate::{
    graphics::{VertexDescription, formats::RGBA},
    ui::{BuildContext, Ui, UiElement, UiRef, UiUnit, materials::AtlasInstance, widget::Widget},
};

pub struct Image {
    pub atlas_index: u32,
    pub max_width: UiUnit,
    pub max_height: UiUnit,
    pub color: RGBA,
    pub stretch: bool,
}

impl Widget for Image {
    fn build(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        context.apply_data(context.child_start_pos, context.available_size);
    }

    fn instance(&mut self, element: UiRef, ui: &mut Ui, clip: Option<Rect2D>) -> Option<Rect2D> {
        let material = &mut ui.materials[2];
        let atlas_entry = &ui.texture_atlas.images[self.atlas_index as usize];
        let to_add = AtlasInstance {
            color: self.color,
            pos: element.pos,
            size: element.size,
            uv_start: atlas_entry.uv_start,
            uv_size: atlas_entry.uv_size,
            z_index: element.z_index,
        };
        material.add(to_add.to_add(), 0, clip);
        clip
    }
}

impl Default for Image {
    fn default() -> Self {
        Self {
            color: RGBA::WHITE,
            atlas_index: 0,
            max_width: UiUnit::Undefined,
            max_height: UiUnit::Undefined,
            stretch: false,
        }
    }
}
