use ash::vk::Rect2D;

use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
    ui::{
        CallContext, FlexDirection, QueuedEvent, Ui, UiEvent, UiRef, materials::UiInstance,
        ui::InputResult, widget::Widget,
    },
};

#[derive(Debug)]
pub struct Button {
    pub margin: UiRect,
    pub padding: UiRect,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub flex_direction: FlexDirection,
    pub border: [u8; 4],
    pub corner: [UiUnit; 4],
    pub state: ButtonState,
    pub callback: Option<fn(CallContext)>,
    pub message: u16,
}

impl Widget for Button {
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
        element: &UiElement,
        ui: &mut Ui,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        let material = &mut ui.materials[0];
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].px(element.size),
            z_index: element.z_index,
        };
        material.add(to_add.to_add(), 0, clip);
        clip
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        let mut result;

        ui.set_event(QueuedEvent::new(&element, event, self.message));

        if event == UiEvent::Press || event == UiEvent::Release {
            result = InputResult::New
        } else if self.state == ButtonState::Normal {
            result = InputResult::New;
        } else {
            result = InputResult::Old;
        };

        match event {
            UiEvent::Press => {
                self.state = ButtonState::Pressed;
                ui.selection.set_hover(&element);
            }
            UiEvent::Release => {
                if element.is_in(ui.cursor_pos) {
                    self.state = ButtonState::Hovered;
                    ui.selection.set_hover(&element);
                } else {
                    result = InputResult::None;
                    self.state = ButtonState::Normal;
                    ui.selection.clear();
                }
            }
            UiEvent::Move => {
                if self.state != ButtonState::Pressed {
                    if result == InputResult::New || element.is_in(ui.cursor_pos) {
                        self.state = ButtonState::Hovered;
                        ui.selection.set_hover(&element);
                    } else {
                        result = InputResult::None;
                        self.state = ButtonState::Normal;
                        ui.selection.clear();
                    }
                }
            }
            UiEvent::End => {
                result = InputResult::New;
                self.state = ButtonState::Normal;
                ui.selection.clear();
            }
            _ => return InputResult::None,
        }

        if let Some(call) = self.callback {
            let context = CallContext { ui, element, event };
            call(context);
        }

        result
    }

    fn has_interaction(&self) -> bool {
        true
    }
}

impl Default for Button {
    fn default() -> Self {
        Self {
            margin: UiRect::default(),
            padding: UiRect::new(5.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Px(5.0); 4],
            flex_direction: FlexDirection::Horizontal,
            state: ButtonState::Normal,
            callback: None,
            message: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}
