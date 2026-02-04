use core::f32;

use ash::vk::Rect2D;
use winit::event::MouseScrollDelta;

use super::{BuildContext, UiElement};
use crate::{
    graphics::Ressources,
    primitives::Vec2,
    ui::{
        FlexDirection, Ui, UiEvent, UiRect, UiRef, UiUnit, system::InputResult, units::FlexAlign,
        widget::Widget,
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
        let space = context.element_size;
        let padding = self.padding.size(context);

        let child_hash: u32 = if let Some(child) = childs.first() {
            child.id
        } else {
            0
        };

        if child_hash != self.child_hash {
            self.scroll_offset.y = 0.0;
            self.child_hash = child_hash;
        }

        let pos = context.pos_child(FlexAlign::default(), Vec2::zero());

        let available_size = space - padding;
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

        // if we resize the element we dont want the scroll offset to be larger than it should be
        if space.y < self.size.y {
            self.scroll_offset.y = self.scroll_offset.y.max(space.y - self.size.y);
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

        // Size must be defined in order to work
        for child in childs {
            child.build_size(&mut child_ctx);
        }

        self.size = child_ctx.final_size() + padding;
        context.apply_size(size);
    }

    fn predict_size(&mut self, context: &mut BuildContext) {
        context.fill_x(1.0);
        context.fill_y(1.0);

        context.predict_child(Vec2::zero());
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        if let UiEvent::Scroll(delta) = event {
            let delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => y * 50.0,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
            };
            let old_offset = self.scroll_offset.y;
            let min = (element.size.y as f32 - self.size.y).min(0.0);

            self.scroll_offset.y += delta;
            self.scroll_offset.y = self.scroll_offset.y.clamp(min, 0.0);

            if old_offset != self.scroll_offset.y {
                ui.color_changed();

                for element in element.childs_mut(ui) {
                    element.offset_element(Vec2::new(0.0, self.scroll_offset.y - old_offset));
                }

                let cursor_pos = ui.cursor_pos;

                // Todo! make this faster by using an extra fn
                ui.handle_input(cursor_pos, UiEvent::Move);
                InputResult::New
            } else {
                InputResult::None
            }
        } else {
            InputResult::None
        }
    }

    fn instance(
        &mut self,
        element: UiRef,
        _: &mut Ressources,
        _: f32,
        _: Option<Rect2D>,
    ) -> Option<Rect2D> {
        Some(Rect2D {
            offset: element.pos.into(),
            extent: element.size.into(),
        })
    }
}
