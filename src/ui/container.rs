use ash::vk::Rect2D;

use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{
        FlexDirection, Ressources, UiRef,
        materials::{MatType, UiInstance},
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
}

impl Widget for Container {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin_start = self.margin.start(context);
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);
        let padding_start = self.padding.start(context);

        let width = match self.width {
            UiUnit::Fill => context.remaining_space().x - margin.x,
            _ => self.width.px(context.available_size - margin),
        };

        let height = match self.height {
            UiUnit::Fill => context.remaining_space().y - margin.y,
            _ => self.height.py(context.available_size - margin),
        };

        let mut size = Vec2::new(width, height);

        let pos = context.pos_child() + margin_start;
        let child_start = pos + padding_start;

        let mut child_ctx =
            BuildContext::new_from(context, size - padding, child_start, self.flex_direction);

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

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(
        &mut self,
        element: UiRef,
        ressources: &mut Ressources,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].px(element.size) as _,
            z_index: element.z_index,
        };
        ressources.add(MatType::Basic, &to_add, clip);
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
        }
    }
}
