use ash::vk::Rect2D;
use winit::event::MouseScrollDelta;

use super::{BuildContext, UiElement, UiUnit};
use crate::{
    primitives::Vec2,
    ui::{FlexDirection, Ui, UiEvent, UiRect, UiRef, ui::InputResult, widget::Widget},
};

#[derive(Default)]
pub struct ScrollPanel {
    pub scroll_offset: Vec2,
    pub size: Vec2,
    pub padding: UiRect,
    pub child_hash: u32,
}

impl Widget for ScrollPanel {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let space = context.remaining_space();
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

        let pos = context.pos_child();

        let available_size = space - padding;
        let child_start_pos = pos + self.padding.start(context);

        let mut child_context = BuildContext::new_from(
            context,
            available_size,
            child_start_pos + self.scroll_offset,
            FlexDirection::Vertical,
        );

        for child in childs {
            child.build(&mut child_context);
        }

        self.size.y = child_context.final_size().y + padding.y;

        // if we resize the element we dont want the scroll offset to be larger than it should be
        if space.y < self.size.y {
            self.scroll_offset.y = self.scroll_offset.y.max(space.y - self.size.y);
        }

        context.apply_data(pos, space);
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        if let UiEvent::Scroll(delta) = event {
            let delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => y * 50.0,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
            };
            let old_offset = self.scroll_offset.y;
            let min = (element.size.y - self.size.y).min(0.0);

            self.scroll_offset.y += delta;
            self.scroll_offset.y = self.scroll_offset.y.clamp(min, 0.0);

            if old_offset != self.scroll_offset.y {
                ui.color_changed();

                for element in element.childs_mut() {
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

    fn instance(&mut self, element: &UiElement, _: &mut Ui, _: Option<Rect2D>) -> Option<Rect2D> {
        Some(Rect2D {
            offset: element.pos.into(),
            extent: element.size.into(),
        })
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Fill, UiUnit::Fill)
    }
}
