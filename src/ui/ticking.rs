use std::time::Instant;

use super::{
    BuildContext, ElementType, UiElement, UiUnit,
    element::{Element, TypeConst},
};
use crate::ui::{CallContext, FnPtr, UiEvent, UiState, ui_state::EventResult};

pub struct Ticking<T: Element + TypeConst> {
    pub last_tick: Instant,
    pub progress: f32,
    pub tick: FnPtr,
    pub inner: T,
}

impl<T: Element + TypeConst> Element for Ticking<T> {
    fn build(&mut self, context: &mut BuildContext) {
        self.inner.build(context);
    }

    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        event: UiEvent,
    ) -> EventResult {
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

    fn add_child(&mut self, child: UiElement) -> Option<&mut UiElement> {
        self.inner.add_child(child)
    }

    fn tick(&mut self, element: &mut UiElement) {
        if !self.tick.is_none() {
            let context = CallContext {
                element,
                event: UiEvent::Tick,
            };
            self.tick.call(context);
        }
    }
}

impl<T: Element + TypeConst> TypeConst for Ticking<T> {
    const ELEMENT_TYPE: ElementType = T::ELEMENT_TYPE;
    const DEFAULT_TICKING: bool = true;
}

impl<T: Element + TypeConst> Default for Ticking<T> {
    fn default() -> Self {
        Self {
            last_tick: Instant::now(),
            progress: 0.0,
            tick: FnPtr::none(),
            inner: T::default(),
        }
    }
}
