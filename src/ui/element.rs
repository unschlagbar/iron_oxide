use std::{fmt::Debug, ptr::{self, NonNull}};

use ash::vk::{self, Rect2D};

use super::{
    Absolute, BuildContext, Button, Container, ElementType, Text,
    UiEvent, UiState, ui_state::EventResult,
};
use crate::{
    primitives::Vec2,
    ui::{ScrollPanel, UiUnit},
};

pub trait Element {
    fn build(&mut self, context: &mut BuildContext, element: &UiElement);

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Undefined, UiUnit::Undefined)
    }

    #[allow(unused)]
    fn instance(&self, element: &UiElement, ui: &mut UiState, clip: Option<Rect2D>) {}

    fn childs_mut(&mut self) -> Option<&mut Vec<UiElement>> {
        None
    }

    fn childs(&self) -> &[UiElement] {
        &[]
    }

    fn add_child(&mut self, child: UiElement) -> Option<&mut UiElement> {
        let childs = self.childs_mut()?;
        childs.push(child);
        childs.last_mut()
    }

    #[allow(unused)]
    fn interaction(&mut self, element: &mut UiElement, ui: &mut UiState, event: UiEvent) -> EventResult {
        EventResult::None
    }

    #[allow(unused)]
    fn tick(&mut self, element: &mut UiElement, ui: &mut UiState) {}
}

pub trait TypeConst: Default + 'static {
    const ELEMENT_TYPE: ElementType;
    const DEFAULT_TICKING: bool = false;
    fn wrap(self, name: &'static str, ui_state: &UiState) -> UiElement
    where
        Self: Element + Sized,
    {
        UiElement {
            id: ui_state.get_id(),
            name,
            typ: Self::ELEMENT_TYPE,
            visible: true,
            size: Vec2::zero(),
            pos: Vec2::zero(),
            parent: None,
            element: Box::new(self),
            z_index: 0.0,
        }
    }
}

pub struct UiElement {
    pub id: u32,
    pub name: &'static str,
    pub typ: ElementType,
    pub visible: bool,
    pub size: Vec2,
    pub pos: Vec2,
    pub z_index: f32,
    pub parent: Option<NonNull<UiElement>>,
    pub element: Box<dyn Element>,
}

impl UiElement {
    #[track_caller]
    pub fn downcast<'a, T: Element + TypeConst>(&'a self) -> &'a T {
        if T::ELEMENT_TYPE != self.typ {
            panic!(
                "Invalid downcast from {:?} to {:?}",
                self.typ,
                T::ELEMENT_TYPE
            );
        } else {
            let raw = ptr::from_ref(self.element.as_ref());
            unsafe { &*(raw as *const T) }
        }
    }

    #[track_caller]
    pub fn downcast_mut<'a, T: Element + TypeConst>(&'a mut self) -> &'a mut T {
        if T::ELEMENT_TYPE != self.typ {
            panic!(
                "Invalid downcast from {:?} to {:?}",
                self.typ,
                T::ELEMENT_TYPE
            );
        } else {
            let raw = ptr::from_mut(self.element.as_mut());
            unsafe { &mut *(raw as *mut T) }
        }
    }

    pub fn parent(&mut self) -> &mut UiElement {
        if let Some(parent) = &mut self.parent {
            unsafe { parent.as_mut() }
        }  else {
            panic!()
        }
    }

    pub fn build(&mut self, context: &mut BuildContext) {
        let element = unsafe { ptr::from_ref(self).as_ref().unwrap() };
        match &self.typ {
            ElementType::Block => {
                let div: &mut Container = self.downcast_mut();
                div.build(context, element);
            }
            ElementType::AbsoluteLayout => {
                let div: &mut Absolute = self.downcast_mut();
                div.build(context, element);
            }
            ElementType::Button => {
                let div: &mut Button = self.downcast_mut();
                div.build(context, element);
            }
            ElementType::Text => {
                let div: &mut Text = self.downcast_mut();
                div.build(context, element);
            }
            ElementType::ScrollPanel => {
                let div: &mut ScrollPanel = self.downcast_mut();
                div.build(context, element);
            }
            _ => unimplemented!(),
        }

        self.pos = context.element_pos;
        self.size = context.element_size;
    }

    pub fn get_instances(&mut self, ui: &mut UiState, clip: Option<vk::Rect2D>) {
        if self.visible {
            if self.typ == ElementType::Text {
                let size = self.parent().size;
                let pos = self.parent().pos;
                let element = unsafe { &*ptr::from_mut(self) };
                let text: &mut Text = self.downcast_mut();
                text.get_font_instances(size, pos, ui, element, clip);
            } else {
                self.element.instance(self, ui, clip);
            }
        }

        let clip = if self.typ == ElementType::ScrollPanel {
            if clip.is_some() {
                panic!("Nested scroll panels are not allowed");
            }
            Some(vk::Rect2D {
                offset: vk::Offset2D {
                    x: self.pos.x as _,
                    y: self.pos.y as _,
                },
                extent: vk::Extent2D {
                    width: self.size.x as _,
                    height: self.size.y as _,
                },
            })
        } else {
            clip
        };

        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.get_instances(ui, clip);
            }
        }
    }

    pub fn get_offset(&mut self) -> Vec2 {
        let mut offset = Vec2::default();
        if let Some(parent) = &mut self.parent {
            let id = self.id;
            let parent = unsafe { parent.as_mut() };
    
            for child in parent.element.childs_mut().unwrap() {
                if child.id == id {
                    break;
                }
                offset = child.pos - parent.pos + child.size;
            }
        }
        offset
    }

    pub fn move_element(&mut self, amount: Vec2) {
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.move_element(amount);
            }
        }
        self.pos += amount;

        if self.typ == ElementType::Text {
            let text: &mut Text = self.downcast_mut();
            for i in &mut text.font_instances {
                i.pos += amount;
            }
        }
    }

    #[inline]
    pub fn is_in(&self, pos: Vec2) -> bool {
        if self.pos <= pos {
            if self.pos.x + self.size.x >= pos.x && self.pos.y + self.size.y >= pos.y {
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
        let text_element: &Text = child.downcast();
        Some(&text_element.text)
    }

    pub fn update_cursor(&mut self, ui: &mut UiState, event: UiEvent) -> EventResult {
        if !self.visible {
            return EventResult::None;
        }

        if self.is_in(ui.cursor_pos) {
            if let Some(childs) = self.element.childs_mut() {
                for child in childs.iter_mut().rev() {
                    let result = child.update_cursor(ui, event);
                    if !result.is_none() {
                        return result;
                    } else if child.typ == ElementType::AbsoluteLayout {
                        return result;
                    };
                }
            }

            let mut result = EventResult::None;
            let element = unsafe { ptr::from_mut(self).as_mut().unwrap() };

            if self.typ.has_interaction() {
                result = self.element.interaction(element, ui, event);
            }

            return result;
        }
        EventResult::None
    }

    pub fn add_to_parent(mut self, parent: &mut UiElement) {
        self.parent = Some(NonNull::from_mut(parent));
        parent.add_child(self);
    }

    pub fn set_parent(&mut self, parent: NonNull<UiElement>) {
        self.parent = Some(parent);

        let parent = NonNull::from_mut(self);
        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.set_parent(parent);
            }
        }
    }

    pub fn add_child(&mut self, mut child: UiElement) -> Option<&mut UiElement> {
        child.parent = Some(NonNull::from_mut(self));
        self.element.add_child(child)
    }

    pub fn clear_childs(&mut self) {
        if let Some(childs) = self.element.childs_mut() {
            childs.clear();
        }
    }

    pub fn remove_self(&mut self, ui: &mut UiState) -> Option<UiElement> {
        ui.remove_element(self)
    }

    pub fn remove_tick(&mut self, ui: &mut UiState) {
        ui.remove_tick(self.id);
        ui.selection.check_removed(self.id);

        if let Some(childs) = self.element.childs_mut() {
            for child in childs {
                child.remove_tick(ui);
            }
        }
    }

    pub fn init(&mut self) {
        let parent = Some(NonNull::from_mut(self));
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
            .field("name", &self.name)
            .field("typ", &self.typ)
            .field("visible", &self.visible)
            .field("size", &self.size)
            .field("pos", &self.pos)
            .finish()
    }
}
