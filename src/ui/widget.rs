use std::any::Any;

use winit::event::KeyEvent;

use crate::{
    graphics::Ressources,
    primitives::Vec2,
    ui::{
        BuildContext, InputResult, Ui, UiElement, UiEvent, UiRef,
        element::{DrawInfo, ElementFlags},
    },
};

#[allow(unused)]
pub trait Widget: Any + 'static {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext);

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {}

    fn predict_size(&mut self, context: &mut BuildContext) {}

    fn draw_data(&mut self, element: UiRef, ressources: &mut Ressources, info: &mut DrawInfo) {}

    fn interaction(&mut self, element: UiRef, ui: &mut Ui, event: UiEvent) -> InputResult {
        InputResult::None
    }

    fn key_event(&mut self, element: UiRef, ui: &mut Ui, event: &KeyEvent) -> InputResult {
        InputResult::None
    }

    fn tick(&mut self, element: UiRef, ui: &mut Ui) {}

    fn is_ticking(&self) -> bool {
        false
    }
}

pub trait ElementBuilder: Default + Widget + Sized + 'static {
    fn wrap_childs(self, name: &'static str, childs: Vec<UiElement>) -> UiElement {
        UiElement {
            id: u32::MAX,
            name,
            flags: ElementFlags::default(),
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            childs,
            widget: Box::new(self),
            z_index: 0,
        }
    }

    fn wrap_childs_transparent(self, name: &'static str, childs: Vec<UiElement>) -> UiElement {
        UiElement {
            id: u32::MAX,
            name,
            flags: ElementFlags::Transparent | ElementFlags::Visible,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            childs,
            widget: Box::new(self),
            z_index: 0,
        }
    }

    fn wrap(self, name: &'static str) -> UiElement {
        self.wrap_childs(name, Vec::new())
    }

    fn wrap_transparent(self, name: &'static str) -> UiElement {
        self.wrap_childs_transparent(name, Vec::new())
    }
}

impl<T: Default + Widget + Sized + 'static> ElementBuilder for T {}

impl Widget for () {
    fn build_layout(&mut self, _: &mut [UiElement], _: &mut BuildContext) {}
}
