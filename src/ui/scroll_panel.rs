use winit::event::MouseScrollDelta;

use super::{
    BuildContext, ElementType, UiElement, UiUnit,
    ui_element::{Element, ElementBuild, TypeConst},
};
use crate::{
    primitives::Vec2,
    ui::{
        DirtyFlags, FlexDirection, OutArea, UiEvent, UiState, draw_data::DrawData,
        ui_state::EventResult,
    },
};

#[derive(Default)]
pub struct ScrollPanel {
    pub scroll_offset: Vec2,
    pub size: Vec2,
    pub padding: OutArea,
    pub child_hash: u32,
    pub childs: Vec<UiElement>,
}

impl Element for ScrollPanel {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement) {
        let space = context.available_size;

        let child_hash: u32 = if let Some(child) = self.childs.first() {
            child.id
        } else {
            0
        };

        if child_hash != self.child_hash {
            self.scroll_offset.y = 0.0;
            self.child_hash = child_hash;
        }

        let available_size = element.size - self.padding.size(space);
        let child_start_pos = context.child_start_pos + self.padding.start(space);

        let mut child_context = BuildContext::new_from(
            context,
            Vec2::new(available_size.x, f32::MAX),
            child_start_pos + self.scroll_offset,
            element,
            FlexDirection::Vertical,
        );

        for element in self.childs.iter_mut() {
            element.build(&mut child_context);
            child_context.order += 1;
        }

        self.size.y = child_context.start_pos.y + self.padding.size(space).y;

        context.apply_data(context.child_start_pos, element.size);
    }

    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        event: UiEvent,
    ) -> EventResult {
        match event {
            UiEvent::Scroll(delta) => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 50.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                let old_offset = self.scroll_offset.y;
                let min = (element.size.y - self.size.y).min(0.0);

                self.scroll_offset.y += delta;
                self.scroll_offset.y = self.scroll_offset.y.clamp(min, 0.0);

                if old_offset != self.scroll_offset.y {
                    ui.dirty = DirtyFlags::Color;

                    for element in &mut self.childs {
                        element.move_element(Vec2::new(0.0, self.scroll_offset.y - old_offset));
                    }
                    
                    let result = ui.check_selected(UiEvent::Move);
                    if !result.is_none() {
                        return EventResult::New;
                    }
                    
                    for element in &mut self.childs {
                        let r = element.update_cursor(ui, UiEvent::Move);
                        if !r.is_none() {
                            break;
                        }
                    }
                }
            }
            _ => return EventResult::None,
        }

        EventResult::New
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Fill, UiUnit::Fill)
    }

    fn instance(&self, _: &UiElement, _: &mut DrawData) {
        ()
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
}

impl ElementBuild for ScrollPanel {
    fn wrap(self, ui_state: &super::UiState) -> UiElement {
        UiElement {
            id: ui_state.get_id(),
            typ: Self::ELEMENT_TYPE,
            dirty: true,
            visible: false,
            size: Vec2::new(0.0, 0.0),
            pos: Vec2::new(0.0, 0.0),
            parent: std::ptr::null_mut(),
            element: Box::new(self),
            z_index: 0.0,
        }
    }
}

impl TypeConst for ScrollPanel {
    const ELEMENT_TYPE: ElementType = ElementType::ScrollPanel;
}
