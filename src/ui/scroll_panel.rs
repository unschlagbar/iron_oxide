use core::f32;

use winit::event::MouseScrollDelta;

use super::{BuildContext, UiElement};
use crate::{
    graphics::Ressources,
    primitives::Vec2,
    ui::{
        DrawInfo, FlexDirection, Ui, UiEvent, UiRect, UiRef, UiUnit, system::InputResult,
        units::FlexAlign, widget::Widget,
    },
};

#[derive(Default)]
pub struct ScrollPanel {
    pub scroll_offset: Vec2<f32>,
    pub size: Vec2<f32>,
    pub padding: UiRect,
    pub child_hash: u32,
}

impl Widget for ScrollPanel {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let available_size = context.element_size - self.padding.size(context);
        let pos = context.pos_child(FlexAlign::default(), Vec2::zero());
        let child_start_pos = pos + self.padding.start(context);

        let mut child_context = BuildContext::child(
            context,
            available_size,
            child_start_pos + self.scroll_offset,
            FlexDirection::Vertical,
        );

        for child in childs {
            child.build(&mut child_context);
        }

        context.apply_pos(pos);
    }

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let padding = self.padding.size(context);

        let size = Vec2::new(
            UiUnit::Fill(1.0).size_x(context),
            UiUnit::Fill(1.0).size_y(context),
        );

        let mut child_ctx = context.child(size - padding, Vec2::zero(), FlexDirection::default());

        for child in &mut *childs {
            child.predict_size(&mut child_ctx);
        }

        for child in &mut *childs {
            child.build_size(&mut child_ctx);
        }

        self.size = child_ctx.final_size() + padding;

        let child_hash = childs.first().map(|c| c.id).unwrap_or_default();

        if child_hash != self.child_hash {
            self.scroll_offset.y = 0.0;
            self.child_hash = child_hash;
        }
        let min = (size.y - self.size.y).min(0.0);
        self.scroll_offset.y = self.scroll_offset.y.clamp(min, 0.0);

        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        context.fill_x(1.0);
        context.fill_y(1.0);
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        if let UiEvent::Scroll(delta) = event {
            let delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => y * 50.0,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
            };
            let old_offset = self.scroll_offset.y;
            self.scroll_offset.y += delta;

            let min = (element.size.y as f32 - self.size.y).min(0.0);
            self.scroll_offset.y = self.scroll_offset.y.clamp(min, 0.0);

            if old_offset != self.scroll_offset.y {
                ui.color_changed();

                for element in element.childs_mut(ui) {
                    element.offset_element(Vec2::new(0.0, self.scroll_offset.y - old_offset));
                }

                ui.handle_input(ui.cursor_pos, UiEvent::Move);
                InputResult::New
            } else {
                InputResult::None
            }
        } else {
            InputResult::None
        }
    }

    fn draw_data(&mut self, element: UiRef, _: &mut Ressources, info: &mut DrawInfo) {
        if element.size.y < self.size.y as i16 {
            info.clip(element.pos.into_f32(), element.size.into_f32());
        }
    }
}
