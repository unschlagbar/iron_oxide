use std::time::Instant;

use ash::vk::Rect2D;

use super::{BuildContext, UiElement, UiUnit};
use crate::{
    graphics::Ressources,
    ui::{ButtonContext, Ui, UiEvent, UiRef, system::InputResult, widget::Widget},
};

pub struct Ticking<T: Widget> {
    pub time: Instant,
    pub progress: f32,
    pub tick: Option<fn(ButtonContext)>,
    pub inner: T,
}

impl<T: Widget> Widget for Ticking<T> {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {
        self.inner.build(childs, context);
    }

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        self.inner.interaction(element, ui, event)
    }

    fn build_size(&mut self) -> (UiUnit, UiUnit) {
        self.inner.build_size()
    }

    fn instance(
        &mut self,
        element: UiRef,
        ressources: &mut Ressources,
        scale_factor: f32,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        self.inner.instance(element, ressources, scale_factor, clip)
    }

    fn tick(&mut self, element: UiRef, ui: &mut Ui) {
        if let Some(call) = self.tick {
            let context = ButtonContext {
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

impl<T: Widget + Default> Default for Ticking<T> {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            progress: 0.0,
            tick: None,
            inner: T::default(),
        }
    }
}
