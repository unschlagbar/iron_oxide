use pyronyx::vk::{Extent2D, Offset2D, Rect2D};

use super::{BuildContext, DrawInfo, UiElement, UiRect, UiUnit};
use crate::{
    graphics::Resources,
    primitives::Vec2,
    ui::{UiRef, widget::Widget},
};

/// A hole in the ui: it takes part in layout like any other box and draws
/// nothing at all. When the draw order reaches it, the application is handed its
/// rectangle and fills it with its own pipeline — see `Resources::draw_with`.
///
/// That is the whole element. It has no children, no colour and no input: what
/// goes inside is not made of ui elements, and a canvas that tried to describe
/// it would only be in the way.
pub struct Canvas {
    pub width: UiUnit,
    pub height: UiUnit,
    pub margin: UiRect,
    /// Which drawing this is. The application matches on it in its callback,
    /// the same way a button's `message` is what a click means.
    pub id: u16,
}

impl Widget for Canvas {
    fn build_layout(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let margin = self.margin.size(context);
        let size = context.element_size;
        let pos = context.pos_child(Default::default(), size) + self.margin.start(context);

        context.place(size + margin);
        context.apply_pos(pos);
    }

    fn build_size(&mut self, _: &mut [UiElement], context: &mut BuildContext) {
        let margin = self.margin.size(context);
        let size = Vec2::new(
            self.width.size_x(context, margin.x),
            self.height.size_y(context, margin.y),
        );

        context.place(size + margin);
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        let size = Vec2::new(
            self.width.pre_size_x(context),
            self.height.pre_size_y(context),
        );
        context.predict(size + self.margin.size(context));
    }

    fn draw_data(&mut self, element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {
        // Layout is already in physical pixels — `UiUnit::Px` scales on the way
        // in — which is the space both a scissor and the projection are in.
        let area = Rect2D {
            offset: Offset2D {
                x: element.pos.x as i32,
                y: element.pos.y as i32,
            },
            extent: Extent2D {
                width: element.size.x as u32,
                height: element.size.y as u32,
            },
        };

        resources.mark(self.id, area, info);
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            width: UiUnit::Fill(1.0),
            height: UiUnit::Fill(1.0),
            margin: UiRect::default(),
            id: 0,
        }
    }
}
