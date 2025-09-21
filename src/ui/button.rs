use super::{
    BuildContext, ElementType, ErasedFnPointer, OutArea, Overflow, RawUiElement, UiElement, UiUnit,
    ui_element::{Element, ElementBuild, TypeConst},
};
use crate::{
    graphics::formats::Color,
    primitives::Vec2,
    ui::{
        CallContext, FlexDirection, QueuedEvent, UiEvent, UiState,
        draw_data::{DrawData, InstanceData},
        ui_state::EventResult,
    },
};

pub struct Button {
    pub margin: OutArea,
    pub padding: OutArea,
    pub overflow: Overflow,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: Color,
    pub border_color: Color,
    pub flex_direction: FlexDirection,
    pub border: [f32; 4],
    pub corner: [UiUnit; 4],
    pub state: ButtonState,
    pub callback: ErasedFnPointer,
    pub message: u16,
    pub comp: RawUiElement,
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

        let comp = &mut self.comp;
        comp.border = self.border[0];
        comp.corner = self.corner[0].pixelx(size);
        comp.size = size;
        comp.pos = pos;

        let available_size = size - self.padding.size(space);
        let child_start_pos = pos + self.padding.start(space);

        let mut child_context = BuildContext::new_from(
            context,
            available_size,
            child_start_pos,
            &comp,
            self.flex_direction,
        );

        for element in self.childs.iter_mut() {
            let (width, height) = element.element.get_size();
            if matches!(width, UiUnit::Fill) {
                element.size.x = (child_context.available_size.x - child_context.start_pos.x).abs();
            }

            if matches!(height, UiUnit::Fill) {
                element.size.y = (child_context.available_size.y - child_context.start_pos.y).abs();
            }
            element.build(&mut child_context);
            child_context.order += 1;
        }

        context.apply_data(pos, size);
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (self.width, self.height)
    }

    fn instance(&self, element: &UiElement, draw_data: &mut DrawData) {
        if let InstanceData::Basic(vec) = draw_data.get_group(0, 0) {
            vec.push(
                self.comp
                    .to_instance(self.color, self.border_color, element.z_index),
            );
        } else {
            unreachable!()
        }
    }

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        Some(&mut self.childs)
    }

    fn childs(&self) -> &[UiElement] {
        &self.childs
    }

    fn add_child(&mut self, child: UiElement) {
        self.childs.push(child);
    }

    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        event: UiEvent,
    ) -> EventResult {
        let this: &mut Self = self;
        let mut result;

        ui.set_event(QueuedEvent::new(element, event, this.message));

        if matches!(this.state, ButtonState::Normal | ButtonState::Disabled) {
            result = EventResult::New;
        } else {
            result = EventResult::Old;
        };

        match event {
            UiEvent::Press => {
                this.state = ButtonState::Pressed;
                ui.selected.set_pressed(element as _);
            }
            UiEvent::Release => {
                if element.is_in(ui.cursor_pos) {
                    this.state = ButtonState::Hovered;
                    ui.selected.set_selected(element);
                } else {
                    result = EventResult::None;
                    this.state = ButtonState::Normal;
                    ui.selected.clear();
                }
            }
            UiEvent::Move => {
                if !matches!(this.state, ButtonState::Pressed) {
                    if matches!(result, EventResult::New) || element.is_in(ui.cursor_pos) {
                        this.state = ButtonState::Hovered;
                        ui.selected.set_selected(element);
                    } else {
                        result = EventResult::None;
                        this.state = ButtonState::Normal;
                        ui.selected.clear();
                    }
                }
            }
            _ => return EventResult::None,
        }

        if !this.callback.is_null() {
            let context = CallContext { ui, element, event };
            this.callback.call(context);
        }

        result
    }
}

impl ElementBuild for Button {
    fn wrap(self, ui_state: &super::UiState) -> UiElement {
        UiElement {
            id: ui_state.get_id(),
            typ: ElementType::Button,
            dirty: true,
            visible: true,
            size: Vec2::new(0.0, 0.0),
            pos: Vec2::new(0.0, 0.0),
            parent: std::ptr::null_mut(),
            element: Box::new(self),
            z_index: 0.0,
        }
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
            overflow: Overflow::hidden(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: Color::DARKGREY,
            border_color: Color::GREEN,
            border: [0.0; 4],
            corner: [UiUnit::Px(5.0); 4],
            flex_direction: FlexDirection::Horizontal,
            state: ButtonState::Normal,
            comp: Default::default(),
            childs: Default::default(),
            callback: ErasedFnPointer::null(),
            message: 0,
        }
    }
}

pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}
