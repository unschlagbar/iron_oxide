use std::{
    any::TypeId,
    fmt::{self, Debug},
    ptr::{self, NonNull},
};

use ash::vk::Rect2D;

use super::{BuildContext, Text, UiEvent, UiState, ui_state::EventResult};
use crate::{
    primitives::Vec2,
    ui::{UiRef, UiUnit},
};

pub trait Element: 'static {
    fn build(&mut self, childs: &mut [UiElement], context: &mut BuildContext);

    fn get_size(&mut self) -> (UiUnit, UiUnit) {
        (UiUnit::Undefined, UiUnit::Undefined)
    }

    #[allow(unused)]
    fn instance(
        &mut self,
        element: &UiElement,
        ui: &mut UiState,
        clip: Option<Rect2D>,
    ) -> Option<Rect2D> {
        clip
    }

    #[allow(unused)]
    fn interaction(&mut self, element: UiRef, ui: &mut UiState, event: UiEvent) -> EventResult {
        EventResult::None
    }

    #[allow(unused)]
    fn has_interaction(&self) -> bool {
        false
    }

    #[allow(unused)]
    fn tick(&mut self, element: UiRef, ui: &mut UiState) {}

    #[allow(unused)]
    fn is_ticking(&self) -> bool {
        false
    }
}

pub trait ElementBuilder: Default + Element + Sized + 'static {
    fn wrap_childs(self, name: &'static str, childs: Vec<UiElement>) -> UiElement {
        UiElement {
            id: u32::MAX,
            name,
            visible: true,
            size: Vec2::zero(),
            pos: Vec2::zero(),
            parent: None,
            childs,
            element: Box::new(self),
            z_index: 0.0,
            type_id: TypeId::of::<Self>(),
        }
    }

    fn wrap(self, name: &'static str) -> UiElement {
        self.wrap_childs(name, Vec::new())
    }
}

impl<T: Default + Element + Sized + 'static> ElementBuilder for T {}

pub struct UiElement {
    pub(crate) id: u32,
    pub name: &'static str,
    pub visible: bool,
    pub size: Vec2,
    pub pos: Vec2,
    pub z_index: f32,
    pub parent: Option<NonNull<Self>>,
    pub childs: Vec<Self>,
    pub element: Box<dyn Element>,
    pub type_id: TypeId,
}

impl UiElement {
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[track_caller]
    pub fn downcast<T: Element>(&self) -> Option<&T> {
        if TypeId::of::<T>() == self.type_id {
            let gg = unsafe { &*((&*self.element) as *const dyn Element as *const T) };
            Some(gg)
        } else {
            None
        }
    }

    #[track_caller]
    pub fn downcast_mut<T: Element>(&mut self) -> Option<&mut T> {
        if TypeId::of::<T>() == self.type_id {
            let gg = unsafe { &mut *((&mut *self.element) as *mut dyn Element as *mut T) };
            Some(gg)
        } else {
            None
        }
    }

    pub(crate) fn childs_mut<'b>(&self) -> &'b mut [Self] {
        let childs: &[Self] = &self.childs;
        #[allow(invalid_reference_casting)]
        unsafe {
            &mut *(childs as *const [Self] as *mut [Self])
        }
    }

    pub fn build(&mut self, context: &mut BuildContext) {
        context.z_index = self.z_index;
        context.element_pos = self.pos;
        context.element_size = self.size;

        let childs = self.childs_mut();

        self.element.build(childs, context);

        self.pos = context.element_pos;
        self.size = context.element_size;
    }

    pub fn get_instances(&mut self, ui: &mut UiState, clip: Option<Rect2D>) {
        let mut inner_clip = clip;

        if self.visible {
            let element = unsafe { &mut *ptr::from_mut(self) };
            inner_clip = self.element.instance(element, ui, clip);
        }

        for child in &mut self.childs {
            child.get_instances(ui, inner_clip);
        }
    }

    pub fn get_offset(&mut self) -> Vec2 {
        let mut offset = Vec2::default();
        if let Some(mut parent) = self.parent {
            let id = self.id;
            let parent = unsafe { parent.as_mut() };

            for child in &mut parent.childs {
                if child.id == id {
                    break;
                }
                offset = child.pos - parent.pos + child.size;
            }
        }
        offset
    }

    pub fn move_element(&mut self, amount: Vec2) {
        self.pos += amount;

        if let Some(text) = self.downcast_mut::<Text>() {
            for i in &mut text.font_instances {
                i.pos += amount;
            }
        }

        for child in &mut self.childs {
            child.move_element(amount);
        }
    }

    #[inline]
    pub fn is_in(&self, pos: Vec2) -> bool {
        self.pos <= pos && self.pos.x + self.size.x >= pos.x && self.pos.y + self.size.y >= pos.y
    }

    pub fn get_text(&self) -> Option<&str> {
        self.get_text_at_pos(0)
    }

    pub fn get_child(&mut self, index: usize) -> Option<UiRef> {
        Some(UiRef::new(self.childs.get_mut(index)?))
    }

    pub fn get_first_child(&mut self) -> Option<UiRef> {
        self.get_child(0)
    }

    pub fn get_text_at_pos(&self, pos: usize) -> Option<&str> {
        let child = self.childs.get(pos)?;
        if let Some(text) = child.downcast::<Text>() {
            Some(&text.text)
        } else {
            None
        }
    }

    pub fn add_text(&mut self, text: Text) {
        let text = text.wrap("");
        self.add_child(text);
    }

    pub fn update_cursor(&mut self, ui: &mut UiState, event: UiEvent) -> EventResult {
        if !self.visible {
            return EventResult::None;
        }

        if self.is_in(ui.cursor_pos) {
            if !self.childs.iter().any(|c| c.id == ui.selection.hover_id()) {
                for child in &mut self.childs {
                    let result = child.update_cursor(ui, event);
                    if !result.is_none() {
                        return result;
                    };
                }
            }

            let mut result = EventResult::None;
            let element = UiRef::new(self);

            if self.element.has_interaction() {
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
        for child in &mut self.childs {
            child.set_parent(parent);
        }
    }

    pub fn add_child(&mut self, child: UiElement) -> Option<UiRef> {
        let parent = Some(NonNull::from_mut(self));
        let childs = &mut self.childs;
        let ptr = childs.as_ptr();

        childs.push(child);

        // if a realloc happens we need to update child ptrs
        if ptr == childs.as_ptr() {
            for child in childs.iter_mut() {
                child.parent = parent;
            }

            Some(UiRef::new(childs.last_mut()?))
        // otherwise we only set the parrent for the new node
        } else {
            let child = childs.last_mut()?;
            child.parent = parent;
            Some(UiRef::new(child))
        }
    }

    pub fn clear_childs(&mut self) {
        self.childs.clear();
    }

    pub fn remove_tick(&mut self, ui: &mut UiState) {
        ui.remove_tick(self.id);
        ui.selection.check_removed(self.id);

        for child in &mut self.childs {
            child.remove_tick(ui);
        }
    }

    pub(crate) fn init(&mut self, ui: &UiState) {
        let parent = Some(NonNull::from_mut(self));
        let z_index = self.z_index + 0.01;
        for child in &mut self.childs {
            child.parent = parent;
            child.id = ui.get_id();
            child.z_index = z_index;
            child.init(ui);
        }
    }

    pub fn get_child_by_id(&self, id: u32) -> Option<UiRef> {
        for child in &self.childs {
            if child.id == id {
                return Some(UiRef::new_ref(child));
            } else {
                let result = child.get_child_by_id(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn get_child_by_id_mut(&mut self, id: u32) -> Option<&mut UiElement> {
        for child in &mut self.childs {
            if child.id == id {
                return Some(child);
            } else {
                let result = child.get_child_by_id_mut(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }
}

impl Debug for UiElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiElement")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("visible", &self.visible)
            .field("size", &self.size)
            .field("pos", &self.pos)
            .finish()
    }
}

impl Element for () {
    fn build(&mut self, _: &mut [UiElement], _: &mut BuildContext) {}
}
