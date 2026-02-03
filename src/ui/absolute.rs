use ash::vk::Rect2D;

use super::{Align, BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        FlexDirection, UiRef,
        materials::{MatType, ShadowInstance, UiInstance},
        style::Shadow,
        widget::Widget,
    },
};

pub struct Absolute {
    pub align: Align,
    pub offset: Vec2<f32>,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub shadow: Shadow,
    pub padding: UiRect,
}

impl Widget for Absolute {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let size = context.element_size;
        let space = context.available_size;

        let pos = context.child_start_pos + self.align.get_pos(space, size, self.offset);

        let padding = self.padding.size(context);

        let mut child_ctx = BuildContext::child(
            context,
            size - padding,
            pos + self.padding.start(context),
            FlexDirection::default(),
        );

        for child in childs {
            child.build(&mut child_ctx);
        }
        context.apply_pos(pos);
    }

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let padding = self.padding.size(context);

        let mut size = Vec2::new(self.width.size_x(context), self.height.size_y(context));

        let mut child_ctx = context.child(size - padding, Vec2::zero(), FlexDirection::default());

        for child in &mut *childs {
            child.predict_size(&mut child_ctx);
        }

        // Size must be defined in order to work
        for child in childs {
            child.build_size(&mut child_ctx);
        }

        if matches!(self.width, UiUnit::Fit) {
            size.x = child_ctx.final_size().x + padding.x;
        }

        if matches!(self.height, UiUnit::Fit) {
            size.y = child_ctx.final_size().y + padding.y;
        }

        context.place_child(size);
        context.apply_size(size);
    }

    fn instance(
        &mut self,
        element: UiRef,
        ressources: &mut Ressources,
        scale_factor: f32,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x,
            y: element.pos.y,
            width: element.size.x,
            height: element.size.y,
            corner: self.corner[0].px_i16(element.size, scale_factor),
            z_index: element.z_index,
        };
        ressources.add(MatType::Basic, &to_add, clip);

        if self.shadow.color != RGBA::ZERO {
            let to_add = ShadowInstance {
                color: self.shadow.color,
                x: element.pos.x + self.shadow.offset.x,
                y: element.pos.y + self.shadow.offset.y,
                width: element.size.x,
                height: element.size.y,
                blur: self.shadow.blur,
                corner: to_add.corner,
                z_index: element.z_index - 1,
            };
            ressources.add(MatType::Shadow, &to_add, clip);
        }

        clip
    }
}

impl Default for Absolute {
    fn default() -> Self {
        Self {
            align: Align::default(),
            offset: Vec2::default(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            shadow: Shadow::default(),
            padding: UiRect::default(),
        }
    }
}
