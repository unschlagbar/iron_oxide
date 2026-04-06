use super::{Align, BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{Resources, formats::RGBA},
    primitives::Vec2,
    ui::{
        FlexAlign, FlexAxis, UiRef,
        element::DrawInfo,
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
        let space = context.available_space;

        let pos = context.child_start_pos + self.align.get_pos(space, size, self.offset);
        let padding = self.padding.size(context);

        let mut child_ctx = BuildContext::child(
            context,
            size - padding,
            pos + self.padding.start(context),
            FlexAxis::default(),
            FlexAlign::Start,
        );

        for child in childs {
            child.build(&mut child_ctx);
        }
        context.apply_pos(pos);
    }

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let padding = self.padding.size(context);

        let mut size = Vec2::new(
            self.width.size_x(context, 0.0),
            self.height.size_y(context, 0.0),
        );

        let mut child_ctx = context.child(
            size - padding,
            Vec2::zero(),
            FlexAxis::default(),
            FlexAlign::Start,
        );

        for child in &mut *childs {
            child.predict_size(&mut child_ctx);
        }

        child_ctx.next();

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

        context.place(size);
        context.apply_size(size);
    }

    fn draw_data(&mut self, element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {
        let corner = self.corner[0].px_16(element.size, info.scale_factor);

        if self.shadow.color != RGBA::ZERO {
            let to_add = ShadowInstance {
                color: self.shadow.color,
                pos: element.pos.into_i16() + self.shadow.offset,
                size: element.size,
                blur: self.shadow.blur,
                corner,
            };
            resources.add(MatType::Shadow, to_add, info);
        }

        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            pos: element.pos.into_i16(),
            size: element.size,
            corner,
        };
        resources.add(MatType::Basic, to_add, info);
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
