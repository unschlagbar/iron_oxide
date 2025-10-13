use super::{
    BuildContext, ElementType, FnPtr, OutArea, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::{
    graphics::formats::RGBA,
    primitives::Vec2,
    ui::{
        CallContext, FlexDirection, QueuedEvent, UiEvent, UiState, materials::UiInstance,
        ui_state::EventResult,
    },
};

pub struct Button {
    pub margin: OutArea,
    pub padding: OutArea,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: RGBA,
    pub border_color: RGBA,
    pub flex_direction: FlexDirection,
    pub border: [i8; 4],
    pub corner: [UiUnit; 4],
    pub state: ButtonState,
    pub callback: FnPtr,
    pub message: u16,
    pub childs: Vec<UiElement>,
}

impl Element for Button {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement) {
        let space = context.available_size;

        let width = if matches!(self.width, UiUnit::Fill) {
            element.size.x
        } else {
            self.width.pixelx(space)
        };
        let height = if matches!(self.height, UiUnit::Fill) {
            element.size.y
        } else {
            self.height.pixely(space)
        };

        let outer_size = Vec2::new(width, height);

        let size = outer_size - self.margin.size(space);
        let mut pos = self.margin.start(space) + context.child_start_pos;

        context.fits_in_line(&mut pos, outer_size);

        let available_size = size - self.padding.size(space);
        let child_start_pos = pos + self.padding.start(space);

        let mut child_context = BuildContext::new_from(
            context,
            available_size,
            child_start_pos,
            element,
            self.flex_direction,
        );

        for element in self.childs.iter_mut() {
            let (width, height) = element.element.get_size();
            if matches!(width, UiUnit::Fill) {
                element.size.x =
                    (child_context.available_size.x - child_context.used_space.x).abs();
            }

            if matches!(height, UiUnit::Fill) {
                element.size.y =
                    (child_context.available_size.y - child_context.used_space.y).abs();
            }
            element.build(&mut child_context);
        }

        context.apply_data(pos, size);
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(&self, element: &UiElement, ui: &mut UiState, clip: Option<ash::vk::Rect2D>) {
        let material = &mut ui.materials[0];
        let to_add = UiInstance {
            color: self.color,
            border_color: self.border_color,
            border: self.border[0],
            x: element.pos.x as _,
            y: element.pos.y as _,
            width: element.size.x as _,
            height: element.size.y as _,
            corner: self.corner[0].pixelx(element.size),
            z_index: element.z_index,
        };
        material.add(&to_add as *const _ as *const _, 0, clip);
    }

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        Some(&mut self.childs)
    }

    fn childs(&self) -> &[UiElement] {
        &self.childs
    }

    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        event: UiEvent,
    ) -> EventResult {
        let mut result;

        ui.set_event(QueuedEvent::new(element, event, self.message));

        if event == UiEvent::Press || event == UiEvent::Release {
            result = EventResult::New
        } else if self.state == ButtonState::Normal {
            result = EventResult::New;
        } else {
            result = EventResult::Old;
        };

        match event {
            UiEvent::Press => {
                self.state = ButtonState::Pressed;
                ui.selection.set_hover(element);
            }
            UiEvent::Release => {
                if element.is_in(ui.cursor_pos) {
                    self.state = ButtonState::Hovered;
                    ui.selection.set_hover(element);
                } else {
                    result = EventResult::None;
                    self.state = ButtonState::Normal;
                    ui.selection.clear();
                }
            }
            UiEvent::Move => {
                if self.state != ButtonState::Pressed {
                    if result == EventResult::New || element.is_in(ui.cursor_pos) {
                        self.state = ButtonState::Hovered;
                        ui.selection.set_hover(element);
                    } else {
                        result = EventResult::None;
                        self.state = ButtonState::Normal;
                        ui.selection.clear();
                    }
                }
            }
            UiEvent::End => {
                result = EventResult::New;
                self.state = ButtonState::Normal;
                ui.selection.clear();
            }
            _ => return EventResult::None,
        }

        if !self.callback.is_none() {
            let context = CallContext { ui, element, event };
            self.callback.call(context);
        }

        result
    }
}

impl TypeConst for Button {
    const ELEMENT_TYPE: ElementType = ElementType::Button;
}

impl Default for Button {
    fn default() -> Self {
        Self {
            margin: OutArea::default(),
            padding: OutArea::new(5.0),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: RGBA::DARKGREY,
            border_color: RGBA::GREEN,
            border: [0; 4],
            corner: [UiUnit::Px(5.0); 4],
            flex_direction: FlexDirection::Horizontal,
            state: ButtonState::Normal,
            childs: Default::default(),
            callback: FnPtr::none(),
            message: 0,
        }
    }
}

#[derive(PartialEq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}
