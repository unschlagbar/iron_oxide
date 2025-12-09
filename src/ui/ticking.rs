use std::time::Instant;

use super::{
    BuildContext, ElementType, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::ui::{CallContext, UiEvent, UiRef, UiState, ui_state::EventResult};

pub struct Ticking<T: Element + TypeConst> {
    pub last_tick: Instant,
    pub progress: f32,
    pub tick: Option<fn(CallContext)>,
    pub inner: T,
}

impl<T: Element + TypeConst> Element for Ticking<T> {
    fn build(&mut self, context: &mut BuildContext) {
        self.inner.build(context);
    }

    fn interaction(&mut self, element: UiRef, ui: &mut UiState, event: UiEvent) -> EventResult {
        self.inner.interaction(element, ui, event)
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        self.inner.get_size()
    }

    fn instance(&self, element: &UiElement, ui: &mut UiState, clip: Option<ash::vk::Rect2D>) {
        self.inner.instance(element, ui, clip);
    }

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        self.inner.childs_mut()
    }

    fn childs(&self) -> &[UiElement] {
        self.inner.childs()
    }

    fn add_child(&mut self, child: UiElement) -> Option<UiRef> {
        self.inner.add_child(child)
    }

    fn tick(&mut self, element: UiRef, ui: &mut UiState) {
        if let Some(call) = self.tick {
            let context = CallContext {
                ui,
                element,
                event: UiEvent::Tick,
            };
            call(context);
        }
    }

    fn is_ticking(&self) -> bool {
        true
    }
}

impl<T: Element + TypeConst> TypeConst for Ticking<T> {
    const ELEMENT_TYPE: ElementType = T::ELEMENT_TYPE;
}

impl<T: Element + TypeConst> Default for Ticking<T> {
    fn default() -> Self {
        Self {
            last_tick: Instant::now(),
            progress: 0.0,
            tick: None,
            inner: T::default(),
        }
    }
}
