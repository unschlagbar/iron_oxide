use super::{
    BuildContext, ElementType, ErasedFnPointer, OutArea, Overflow, RawUiElement, UiElement, UiUnit,
    ui_element::{Element, ElementBuild, TypeConst},
};
use crate::{
    graphics::formats::Color,
    primitives::Vec2,
    ui::{CallContext, UiEvent, UiState, ui_state::EventResult},
};

pub struct Button {
    pub margin: OutArea,
    pub padding: OutArea,
    pub overflow: Overflow,
    pub width: UiUnit,
    pub height: UiUnit,
    pub color: Color,
    pub border_color: Color,
    pub border: [f32; 4],
    pub corner: [UiUnit; 4],
    pub state: ButtonState,
    pub callback: ErasedFnPointer,
    pub comp: RawUiElement,
    pub childs: Vec<UiElement>,
}

impl Element for Button {
    fn build(&mut self, context: &mut BuildContext, _: &UiElement) {
        let mut size;
        let mut pos;

        let space = Vec2::new(
            context.parent_size.x - self.padding.x(context.parent_size),
            context.parent_size.y - self.padding.y(context.parent_size),
        );

        size = Vec2::new(self.width.pixelx(space), self.height.pixely(space));

        let mut outer_size = size
            + Vec2::new(
                self.margin.x(context.parent_size),
                self.margin.y(context.parent_size),
            );

        pos = self.margin.start(context.parent_size);

        context.fits_in_line(&mut pos, &mut outer_size);

        let comp = &mut self.comp;

        comp.border = self.border[0];
        comp.corner = self.corner[0].pixelx(size);

        pos += context.parent_pos;

        comp.size = size;
        comp.pos = pos;

        let mut child_context = BuildContext::new_from(context, size, pos, &comp);

        for element in self.childs.iter_mut() {
            element.build(&mut child_context);
            child_context.order += 1;
        }

        if self.width == UiUnit::Auto && child_context.start_pos.x != 0.0 {
            size.x = child_context.start_pos.x
        }
        if self.height == UiUnit::Auto && child_context.start_pos.y != 0.0 {
            comp.size.y = child_context.start_pos.y + child_context.line_offset
        }

        context.apply_data(pos, size);
    }

    fn instance(&self, element: &UiElement) -> crate::graphics::UiInstance {
        self.comp.to_instance(self.color, self.border_color, element.z_index)
    }

    fn childs(&mut self) -> &mut [UiElement] {
        &mut self.childs
    }

    fn add_child(&mut self, child: UiElement) {
        self.childs.push(child);
    }

    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        cursor_pos: Vec2,
        event: UiEvent,
    ) -> EventResult {
        let button: &mut Button = self;
        let mut result;

        if matches!(button.state, ButtonState::Normal | ButtonState::Disabled) {
            result = EventResult::New;
        } else {
            result = EventResult::Old;
        };

        match event {
            UiEvent::Press => {
                button.state = ButtonState::Pressed;
                ui.selected.set_pressed(element as _);
            }
            UiEvent::Release => {
                if element.is_in(cursor_pos) {
                    button.state = ButtonState::Hovered;
                    ui.selected.set_selected(element);
                } else {
                    result = EventResult::None;
                    button.state = ButtonState::Normal;
                    ui.selected.clear();
                }
            }
            UiEvent::Move => {
                if !matches!(button.state, ButtonState::Pressed) {
                    if matches!(result, EventResult::New) || element.is_in(cursor_pos) {
                        button.state = ButtonState::Hovered;
                        ui.selected.set_selected(element);
                    } else {
                        result = EventResult::None;
                        button.state = ButtonState::Normal;
                        ui.selected.clear();
                    }
                }
            }
        }

        if !button.callback.is_null() {
            let context = CallContext { ui, element, event };
            button.callback.call(context);
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
            padding: OutArea::default(),
            overflow: Overflow::hidden(),
            width: UiUnit::Px(100.0),
            height: UiUnit::Px(100.0),
            color: Color::DARKGREY,
            border_color: Color::GREEN,
            border: [1.0; 4],
            corner: [UiUnit::Px(5.0); 4],
            state: ButtonState::Normal,
            comp: Default::default(),
            childs: Default::default(),
            callback: ErasedFnPointer::null(),
        }
    }
}

pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}
