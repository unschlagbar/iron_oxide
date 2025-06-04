use std::fmt::Debug;

use crate::primitives::Vec2;

use super::{callback::ErasedFnPointer, Align, Interaction, Style, UiElement, UiType};


#[derive(Clone)]
pub struct DragBox {
    pub grip_height: f32,
    pub snap_strenght: f32,
    pub align: Align,
    pub axis: u8,
    pub interaction: Interaction,
    pub move_parrent: bool,
    pub on_drag: ErasedFnPointer
}

impl DragBox {
    pub fn new(style: Style) -> UiElement {
        UiElement::extend(
            style,
            vec![],
            UiType::DragBox(Self { grip_height: 20.0, snap_strenght: 50.0, align: Align::Center, axis: 3, interaction: Interaction::None, move_parrent: true, on_drag: ErasedFnPointer::null() })
        )
    }

    pub fn newx(style: Style) -> UiElement {
        UiElement::extend(
            style,
            vec![],
            UiType::DragBox(Self { grip_height: 20.0, snap_strenght: 50.0, align: Align::Center, axis: 1, interaction: Interaction::None, move_parrent: false, on_drag: ErasedFnPointer::null() })
        )
    }

    pub fn on_drag(&mut self, fp: fn(&mut DragEvent)) {
        self.on_drag = ErasedFnPointer::from_free_vars(fp);
    }
}

impl Default for DragBox {
    fn default() -> Self {
        Self { grip_height: 20.0, snap_strenght: 50.0, align: Align::Center, axis: 3, interaction: Interaction::None, move_parrent: true, on_drag: ErasedFnPointer::null() }
    }
}

impl Debug for DragBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dragbox").field("interaction", &self.interaction).finish()
    }
}

pub struct DragEvent<'a> {
    pub move_vec: Vec2,
    pub element: &'a mut UiElement,
}