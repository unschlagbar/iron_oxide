use std::{fmt::Debug, rc::Rc};

use super::{
    AbsoluteLayout, BuildContext, Button, ButtonState, CallContext, Container, ElementType, Text,
    UiEvent, UiState, ui_state::EventResult,
};
use crate::{graphics::UiInstance, primitives::Vec2, ui::UiUnit};

pub trait Element {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement);
    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Undefined, UiUnit::Undefined)
    }
    fn instance(&self, _element: &UiElement) -> UiInstance {
        unimplemented!()
    }
    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        None
    }
    fn childs(&self) -> &[UiElement] {
        &[]
    }
    fn add_child(&mut self, _child: UiElement) {}
    #[allow(unused)]
    fn interaction(
        &mut self,
        element: &mut UiElement,
        ui: &mut UiState,
        curser_pos: Vec2,
        event: UiEvent,
    ) -> EventResult {
        EventResult::None
    }
}

pub trait ElementBuild {
    fn wrap(self, ui_state: &UiState) -> UiElement;
}

pub trait TypeConst {
    const ELEMENT_TYPE: ElementType;
}

pub struct UiElement {
    pub id: u32,
    pub typ: ElementType,
    pub dirty: bool,
    pub visible: bool,
    pub size: Vec2,
    pub pos: Vec2,
    pub z_index: f32,
    pub parent: *mut UiElement,
    pub element: Box<dyn Element>,
}

impl UiElement {
    pub unsafe fn downcast<'a, T: Element>(&'a self) -> &'a T {
        let raw: *const dyn Element = &*self.element as *const dyn Element;
        unsafe { &*(raw as *const T) }
    }

    pub unsafe fn downcast_mut<'a, T: Element>(&'a mut self) -> &'a mut T {
        let raw: *mut dyn Element = &mut *self.element as *mut dyn Element;
        unsafe { &mut *(raw as *mut T) }
    }

    pub fn parent(&mut self) -> &mut UiElement {
        unsafe { &mut *self.parent }
    }

    pub fn build(&mut self, context: &mut BuildContext) {
        let element = unsafe { &*(self as *const UiElement) };
        match &self.typ {
            ElementType::Block => {
                let div: &mut Container = unsafe { self.downcast_mut() };
                div.build(context, element);
                self.dirty = false;
            }
            ElementType::AbsoluteLayout => {
                let div: &mut AbsoluteLayout = unsafe { self.downcast_mut() };
                div.build(context, element);
                self.dirty = false;
            }
            ElementType::Button => {
                let div: &mut Button = unsafe { self.downcast_mut() };
                div.build(context, element);
                self.dirty = false;
            }
            ElementType::Text => {
                let div: &mut Text = unsafe { self.downcast_mut() };
                div.build(context, element);
                self.dirty = false;
            }
            _ => unimplemented!(),
        }

        self.pos = context.element_pos;
        self.size = context.element_size;
    }

    pub fn end_selection(&mut self, ui: &mut UiState) {
        if self.typ == ElementType::Button {
            let element = unsafe { &mut *(self as *mut UiElement) };
            let button: &mut Button = unsafe { self.downcast_mut() };
            button.state = ButtonState::Normal;
            if !button.callback.is_null() {
                let context = CallContext {
                    ui,
                    element,
                    event: UiEvent::Release,
                };
                button.callback.call(context);
            }
        }
    }

    pub fn get_instances(&mut self, ui: &mut UiState, instances: &mut Vec<UiInstance>) {
        if self.visible {
            if self.typ == ElementType::Text {
                let size = self.parent().size;
                let pos = self.parent().pos;
                let element = unsafe { &*(self as *const UiElement) };
                let text = unsafe { self.downcast_mut::<Text>() };
                text.get_font_instances(size, pos, ui, element);
            } else {
                instances.push(self.element.instance(self));
            }
        }

        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.get_instances(ui, instances);
            }
        }
    }

    #[inline(always)]
    pub fn get_offset(&mut self) -> Vec2 {
        let mut offset = Vec2::default();
        if !self.parent.is_null() {
            let id = self.id;
            let parent = self.parent();

            for child in parent.element.childs_mut().unwrap() {
                if child.id == id {
                    break;
                }
                offset = child.pos - parent.pos + child.size;
            }
        }
        offset
    }

    #[inline(always)]
    pub fn move_computed(&mut self, amount: Vec2) {
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.move_computed(amount);
            }
        }
        self.pos += amount;

        if self.typ == ElementType::Text {
            todo!()
            //for raw in &mut text.comp_text {
            //    raw.x += amount.x;
            //    raw.y += amount.y;
            //}
        }
    }

    #[inline(always)]
    pub fn move_computed_absolute(&mut self, pos: Vec2) {
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.move_computed_absolute(pos);
            }
        }
        self.pos = pos;
    }

    #[inline]
    pub fn is_in(&self, pos: Vec2) -> bool {
        if self.pos < pos {
            if self.pos.x + self.size.x > pos.x && self.pos.y + self.size.y > pos.y {
                return true;
            }
        }
        false
    }

    pub fn get_text(&self) -> Option<&str> {
        let child = self.element.childs().get(0)?;
        if !matches!(child.typ, ElementType::Text) {
            return None;
        }
        let text_element: &Text = unsafe { child.downcast() };
        Some(&text_element.text)
    }

    #[allow(unused)]
    pub fn update_cursor(
        &mut self,
        ui: &mut UiState,
        cursor_pos: Vec2,
        event: UiEvent,
    ) -> EventResult {
        if !self.visible {
            return EventResult::None;
        }

        let (size, pos) = (self.size, self.pos);

        if self.is_in(cursor_pos) {
            if let Some(childs) = self.element.childs_mut() {
                for child in childs {
                    let result = child.update_cursor(ui, cursor_pos, event);
                    if !result.is_none() {
                        return result;
                    };
                }
            }

            let mut result = EventResult::None;
            let element = unsafe { &mut *(self as *mut _) };

            match self.typ {
                ElementType::Button => {
                    result = self.element.interaction(element, ui, cursor_pos, event);
                }
                _ => (),
            }

            return result;
        }
        EventResult::None
    }

    pub fn add_to_parent(mut self, parent: &mut UiElement) {
        self.parent = parent as *mut UiElement;
        parent.add_child(self);
    }

    #[inline]
    pub fn get_mut(this: &mut Rc<UiElement>) -> Option<&mut UiElement> {
        Rc::get_mut(this)
    }

    #[inline]
    pub fn set_dirty(&self) {
        unsafe {
            (self as *const UiElement as *mut UiElement)
                .as_mut()
                .unwrap_unchecked()
                .dirty = true
        };
    }

    #[inline]
    pub fn add_child(&mut self, mut child: UiElement) {
        child.parent = self as *mut UiElement;
        self.element.add_child(child);
    }

    pub fn clear_childs(&mut self) {
        if let Some(childs) = self.element.childs_mut() {
            childs.clear();
        }
    }

    pub fn init(&mut self) {
        let parent = self as *mut UiElement;
        let z_index = self.z_index + 0.01;
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.parent = parent;
                child.z_index = z_index;
                child.init();
            }
        }
    }

    pub fn get_child_by_id(&mut self, id: u32) -> Option<&mut UiElement> {
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                if child.id == id {
                    return Some(child);
                } else {
                    let result = child.get_child_by_id(id);
                    if result.is_some() {
                        return result;
                    }
                }
            }
        }
        None
    }
}

impl Debug for UiElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiElement")
            .field("id", &self.id)
            .field("typ", &self.typ)
            .field("dirty", &self.dirty)
            .field("visible", &self.visible)
            .field("size", &self.size)
            .field("pos", &self.pos)
            .finish()
    }
}
