use crate::{graphics::{formats::Color, UiInstance}, primitives::Vec2};

#[derive(Clone, Debug)]
pub struct RawUiElement {
    pub border: f32,
    pub view: Vec2,
    pub pos: Vec2,
    pub size: Vec2,
    pub corner: f32,
}

impl RawUiElement {

    pub const fn new(pos: Vec2, size: Vec2, border: f32, view: Vec2, corner: f32) -> Self {
        Self { pos, size, border, corner, view }
    }

    #[inline(always)]
    pub fn to_instance(&self, color: Color, border_color: Color) -> UiInstance {
        UiInstance { 
            color: color,
            border_color: border_color,
            border: self.border,
            x: self.pos.x.floor(),
            y: self.pos.y.floor(),
            width: self.size.x.floor(),
            height: self.size.y.floor(),
            corner: self.corner,
        }
    }
}

impl Default for RawUiElement {
    fn default() -> Self {
        Self { pos: Vec2::zero(), size: Vec2::zero(), view: Vec2::zero(), border: 0.0, corner: 0.0 }
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum UiEvent {
    Press,
    Release,
    Move,
}