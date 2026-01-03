use std::time::Instant;

use ash::vk::Rect2D;

use super::{
    BuildContext, UiElement, UiUnit,
    element::{Element, ElementBuilder},
};
use crate::ui::{CallContext, UiEvent, UiRef, UiState, ui_state::EventResult};

pub struct Ticking<T: Element + ElementBuilder> {
    pub last_tick: Instant,
    pub progress: f32,
    pub tick: Option<fn(CallContext)>,
    pub inner: T,
}

impl<T: Element + ElementBuilder> Element for Ticking<T> {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        self.inner.build(childs, context);
    }

    fn interaction(&mut self, element: UiRef, ui: &mut UiState, event: UiEvent) -> EventResult {
        self.inner.interaction(element, ui, event)
    }

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        self.inner.get_size()
    }

    fn instance(
        &mut self,
        element: &UiElement,
        ui: &mut UiState,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        self.inner.instance(element, ui, clip)
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

impl<T: Element + ElementBuilder> Default for Ticking<T> {
    fn default() -> Self {
        Self {
            last_tick: Instant::now(),
            progress: 0.0,
            tick: None,
            inner: T::default(),
        }
    }
}
