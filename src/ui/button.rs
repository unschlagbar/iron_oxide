use winit::window::CursorIcon;

use super::{BuildContext, UiElement, UiRect, UiUnit};
use crate::{
    graphics::{Ressources, formats::RGBA},
    primitives::Vec2,
    ui::{
        ButtonContext, DrawInfo, FlexDirection, QueuedEvent, Shadow, Ui, UiEvent, UiRef,
        materials::{MatType, ShadowInstance, UiInstance},
        system::InputResult,
        units::FlexAlign,
        widget::Widget,
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
    pub shadow: Shadow,
    pub state: ButtonState,
    pub callback: Option<fn(ButtonContext)>,
    /// Requests the pointer cursor on hover
    pub cursor: CursorIcon,
    pub message: u16,
}

impl Widget for Button {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        let margin = self.margin.size(context);
        let padding = self.padding.size(context);

        let size = context.element_size;

        let pos =
            context.pos_child(FlexAlign::default(), Vec2::zero()) + self.margin.start(context);
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

        for child in childs.iter_mut() {
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
                corner
            };
            ressources.add(MatType::Shadow, to_add, info);
        }

        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border,
            pos: element.pos,
            size: element.size,
            corner
        };
        ressources.add(MatType::Basic, to_add, info);
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
            UiEvent::Release | UiEvent::TouchRelease => {
                if is_in {
                    self.state = ButtonState::Hovered;
                } else {
                    self.state = ButtonState::Normal;
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

        if self.state != ButtonState::Normal && is_in {
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
            padding: UiRect::px(5.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Px(5.0); 4],
            shadow: Shadow::default(),
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
