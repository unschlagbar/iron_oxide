use ash::vk::Rect2D;

use super::{Align, BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
    ui::{FlexDirection, Ui, UiRef, materials::UiInstance, widget::Widget},
};

pub struct Absolute {
    pub align: Align,
    pub x: UiUnit,
    pub y: UiUnit,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub padding: UiRect,
}

impl Widget for Absolute {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let space = context.available_size;
        let padding = self.padding.size(context);

        let width = self.width.pixelx(context);
        let height = self.height.pixely(context);

        let mut size = Vec2::new(width, height);

        let pos = context.child_start_pos
            + self.align.get_pos(
                space,
                size,
                Vec2::new(self.x.pixelx(context), self.y.pixely(context)),
            );

        let mut child_ctx = BuildContext::new_from(
            context,
            size - padding,
            pos + self.padding.start(context),
            FlexDirection::default(),
        );

        for child in childs {
            child.build(&mut child_ctx);
        }

        // use autosize if width or height was auto
        if matches!(self.width, UiUnit::Auto) {
            size.x = child_ctx.final_size().x + padding.x;
        }

        if matches!(self.height, UiUnit::Auto) {
            size.y = child_ctx.final_size().y + padding.y;
        }

        context.apply_data(pos, size);
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(&mut self, element: UiRef, ui: &mut Ui, clip: Option<Rect2D>) -> Option<Rect2D> {
        let material = &mut ui.materials[0];
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].px(element.size) as u16,
            z_index: element.z_index,
        };
        material.add(to_add.to_add(), 0, clip);
        clip
    }
}

impl Default for Absolute {
    fn default() -> Self {
        Self {
            align: Align::default(),
            x: UiUnit::Px(10.0),
            y: UiUnit::Px(10.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            padding: UiRect::default(),
        }
    }
}
