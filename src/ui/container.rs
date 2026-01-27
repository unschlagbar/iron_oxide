use ash::vk::Rect2D;

use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        FlexDirection, Shadow, UiRef,
        materials::{MatType, ShadowInstance, UiInstance},
        widget::Widget,
    },
};

pub struct Container {
    pub margin: UiRect,
    pub padding: UiRect,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub flex_direction: FlexDirection,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub shadow: Shadow,
}

impl Widget for Container {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin_start = self.margin.start(context);
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);
        let padding_start = self.padding.start(context);

        let width = match self.width {
            UiUnit::Fill => context.remaining_space().x - margin.x,
            _ => self
                .width
                .px(context.available_size - margin, context.scale_factor),
        };

        let height = match self.height {
            UiUnit::Fill => context.remaining_space().y - margin.y,
            _ => self
                .height
                .py(context.available_size - margin, context.scale_factor),
        };

        let mut size = Vec2::new(width, height);

        let pos = context.pos_child() + margin_start;
        let child_start = pos + padding_start;

        let mut child_ctx =
            BuildContext::new(context, size - padding, child_start, self.flex_direction);

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

        context.place_child(size + margin);
        context.apply_data(pos, size);
    }

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin_start = self.margin.start(context);
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);
        let padding_start = self.padding.start(context);

        let width = match self.width {
            UiUnit::Fill => context.remaining_space().x - margin.x,
            _ => self
                .width
                .px(context.available_size - margin, context.scale_factor),
        };

        let height = match self.height {
            UiUnit::Fill => context.remaining_space().y - margin.y,
            _ => self
                .height
                .py(context.available_size - margin, context.scale_factor),
        };

        let mut size = Vec2::new(width, height);

        let pos = context.pos_child() + margin_start;
        let child_start = pos + padding_start;

        let mut child_ctx =
            BuildContext::new(context, size - padding, child_start, self.flex_direction);

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

        context.place_child(size + margin);
        context.apply_pos(pos);
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
                x: element.pos.x + self.shadow.offset.0,
                y: element.pos.y + self.shadow.offset.0,
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

impl Default for Container {
    fn default() -> Self {
        Self {
            margin: UiRect::default(),
            padding: UiRect::default(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            flex_direction: FlexDirection::default(),
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            shadow: Shadow::new(0, RGBA::ZERO),
        }
    }
}
