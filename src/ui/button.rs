use ash::vk::Rect2D;
use winit::window::CursorIcon;

use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{VertexDescription, formats::RGBA},
    primitives::Vec2,
    ui::{
        ButtonContext, FlexDirection, QueuedEvent, Ui, UiEvent, UiRef, materials::UiInstance,
        system::InputResult, widget::Widget,
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
    pub callback: Option<fn(ButtonContext)>,
    /// if true the ui requests the pointer cursor on hover
    pub cursor: CursorIcon,
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

    fn instance(&mut self, element: UiRef, ui: &mut Ui, clip: Option<Rect2D>) -> Option<Rect2D> {
        let material = &mut ui.materials[0];
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].px(element.size) as u16,
            z_index: element.z_index,
        };
        material.add(to_add.to_add(), 0, clip);
        clip
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        let mut result = InputResult::New;

        let old_state = self.state;
        let is_in = element.is_in(ui.cursor_pos);

        if is_in {
            ui.set_event(QueuedEvent::new(&element, event, self.message));
        }

        match event {
            UiEvent::Press => {
                self.state = ButtonState::Pressed;
                ui.selection.set_capture(element);
            }
            UiEvent::Release => {
                ui.selection.clear_capture();

                if is_in {
                    self.state = ButtonState::Hovered;
                } else {
                    self.state = ButtonState::Normal;
                    ui.selection.clear_hover();
                }
            }
            UiEvent::Move => {
                if self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                }
            }
            UiEvent::HoverEnd => {
                result = InputResult::None;
                self.state = ButtonState::Normal;
            }
            _ => return InputResult::None,
        }

        if self.state != ButtonState::Normal {
            ui.cursor_icon = self.cursor;
        }

        if let Some(call) = self.callback
            && old_state != self.state
        {
            let context = ButtonContext { ui, element, event };
            call(context);
        }

        result
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
            cursor: CursorIcon::Pointer,
            message: 0,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}
