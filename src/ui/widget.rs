use std::any::Any;

use winit::event::KeyEvent;

use crate::{
    graphics::Resources,
    primitives::Vec2,
    ui::{
        BuildContext, InputResult, Ui, UiElement, UiEvent, UiRef,
        element::{DrawInfo, ElementFlags},
    },
};

#[allow(unused)]
pub trait Widget: Any + 'static {
    fn build_layout(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {}

    fn build_size(&mut self, childs: &mut [UiElement], context: &mut BuildContext) {}

    fn predict_size(&mut self, context: &mut BuildContext) {}

    fn draw_data(&mut self, element: UiRef, resources: &mut Resources, info: &mut DrawInfo) {}

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
            id: usize::MAX,
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
            id: usize::MAX,
            name,
            flags: ElementFlags::Transparent,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            childs,
            widget: Box::new(self),
            z_index: 0,
        }
    }

    fn wrap_name(self, name: &'static str) -> UiElement {
        self.wrap_childs(name, Vec::new())
    }

    fn wrap(self) -> UiElement {
        self.wrap_childs("", Vec::new())
    }

    fn wrap_flags(self, name: &'static str, flags: ElementFlags) -> UiElement {
        UiElement {
            id: usize::MAX,
            name,
            flags,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            childs: Vec::new(),
            widget: Box::new(self),
            z_index: 0,
        }
    }

    fn wrap_transparent(self, name: &'static str) -> UiElement {
        self.wrap_childs_transparent(name, Vec::new())
    }
}

impl<T: Default + Widget + Sized + 'static> ElementBuilder for T {}

#[macro_export]
macro_rules! node {
    // --- Einstieg ---
    ($widget:expr $(, $($rest:tt)*)?) => {
        node!(@parse
            widget = $widget,
            name = "",
            flags = iron_oxide::ui::ElementFlags::default(),
            children = [],
            $($($rest)*, )?
        )
    };

    // --- Basisregel ---
    (@parse
        widget = $widget:expr,
        name = $name:expr,
        flags = $flags:expr,
        children = [$($children:expr),*],
    ) => {
        iron_oxide::ui::UiElement::from_raw(
            $name,
            $flags,
            vec![$($children),*],
            $widget
        )
    };

    // --- String-Literal → name ---
    (@parse
        widget = $widget:expr,
        name = $_old_name:expr,
        flags = $flags:expr,
        children = [$($children:expr),*],
        $name:literal, $($rest:tt)*
    ) => {
        node!(@parse
            widget = $widget,
            name = $name,
            flags = $flags,
            children = [$($children),*],
            $($rest)*
        )
    };

    // --- flags(...) → flags ---
    (@parse
        widget = $widget:expr,
        name = $name:expr,
        flags = $_old_flags:expr,
        children = [$($children:expr),*],
        flags($new_flags:expr), $($rest:tt)*
    ) => {
        node!(@parse
            widget = $widget,
            name = $name,
            flags = $new_flags,
            children = [$($children),*],
            $($rest)*
        )
    };

    // --- Ausdruck → Kind-Element ---
    (@parse
        widget = $widget:expr,
        name = $name:expr,
        flags = $flags:expr,
        children = [$($children:expr),*],
        $child:expr, $($rest:tt)*
    ) => {
        node!(@parse
            widget = $widget,
            name = $name,
            flags = $flags,
            children = [$($children,)* $child],
            $($rest)*
        )
    };
}
