use ash::vk::Rect2D;

use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        BuildContext, UiElement, UiRef, UiUnit,
        materials::{AtlasInstance, MatType},
        widget::Widget,
    },
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

    fn instance(
        &mut self,
        element: UiRef,
        ressources: &mut Ressources,
        _: f32,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        let atlas_entry = &ressources.texture_atlas.images[self.atlas_index as usize];
        let to_add = AtlasInstance {
            color: self.color,
            pos: Vec2::new(element.pos.x as f32, element.pos.y as f32),
            size: Vec2::new(element.size.x as f32, element.size.y as f32),
            uv_start: atlas_entry.uv_start,
            uv_size: atlas_entry.uv_size,
            z_index: element.z_index,
        };
        ressources.add(MatType::Atlas, &to_add, clip);
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
