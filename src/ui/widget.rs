use std::any::TypeId;

use ash::vk::Rect2D;
use winit::event::KeyEvent;

use crate::{
    primitives::Vec2,
    ui::{BuildContext, Image, InputResult, Text, Ui, UiElement, UiEvent, UiRef, UiUnit},
};

#[allow(unused)]
pub trait Widget: 'static {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext);

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Undefined, UiUnit::Undefined)
    }

    fn instance(&mut self, element: UiRef, ui: &mut Ui, clip: Option<Rect2D>) -> Option<Rect2D> {
        clip
    }

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
        let transparent = TypeId::of::<Self>() == TypeId::of::<Text>()
            || TypeId::of::<Self>() == TypeId::of::<Image>();
        UiElement {
            id: u32::MAX,
            name,
            visible: true,
            transparent,
            size: Vec2::zero(),
            pos: Vec2::zero(),
            parent: None,
            childs,
            widget: Box::new(self),
            z_index: 0.0,
            type_id: TypeId::of::<Self>(),
        }
    }

    fn wrap(self, name: &'static str) -> UiElement {
        self.wrap_childs(name, Vec::new())
    }
}

impl<T: Default + Widget + Sized + 'static> ElementBuilder for T {}

impl Widget for () {
    fn build(&mut self, _: &mut [UiElement], _: &mut BuildContext) {}
}
