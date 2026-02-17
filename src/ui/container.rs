use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        DrawInfo, FlexDirection, Shadow, UiRef,
        materials::{MatType, ShadowInstance, UiInstance},
        units::FlexAlign,
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
    pub flex_align: FlexAlign,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub shadow: Shadow,
}

impl Widget for Container {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);

        let size = context.element_size;

        let pos = context.pos_child(self.flex_align, size) + self.margin.start(context);
        let child_start = pos + self.padding.start(context);

        let mut child_ctx = context.child(size - padding, child_start, self.flex_direction);

        for child in childs {
            child.build(&mut child_ctx);
        }

        context.place_child(size + margin);
        context.apply_pos(pos);
    }

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);

        let mut size = Vec2::new(self.width.size_x(context), self.height.size_y(context));

        let mut child_ctx = context.child(size - padding, Vec2::zero(), self.flex_direction);

        for child in &mut *childs {
            child.predict_size(&mut child_ctx);
        }

        // Size must be defined in order to work
        //if any element depends on parent size while parent size
        for child in childs {
            child.build_size(&mut child_ctx);
        }

        if matches!(self.width, UiUnit::Fit) {
            size.x = child_ctx.final_size().x + padding.x;
        }

        if matches!(self.height, UiUnit::Fit) {
            size.y = child_ctx.final_size().y + padding.y;
        }

        context.place_child(size + margin);
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        let mut size = Vec2::zero();
        let margin = self.margin.size(context);

        if let UiUnit::Fill(weight) = self.width {
            context.fill_x(weight);
        } else {
            size.x = self.width.pre_size_x(context);
        }

        if let UiUnit::Fill(weight) = self.height {
            context.fill_y(weight);
        } else {
            size.y = self.height.pre_size_y(context);
        }
        context.predict_child(size + margin);
    }

    fn draw_data(&mut self, element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {
        let corner = self.corner[0].px_i16(element.size, info.scale_factor);

        if self.shadow.color != RGBA::ZERO {
            let to_add = ShadowInstance {
                color: self.shadow.color,
                pos: element.pos + self.shadow.offset,
                size: element.size,
                blur: self.shadow.blur,
                corner,
            };
            ressources.add(MatType::Shadow, to_add, info);
        }

        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            pos: element.pos,
            size: element.size,
            corner,
        };
        ressources.add(MatType::Basic, to_add, info);
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
            flex_align: FlexAlign::default(),
            border: [0; 4],
            corner: [UiUnit::Zero; 4],
            shadow: Shadow::default(),
        }
    }
}
