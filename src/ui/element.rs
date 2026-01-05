use std::{
    any::TypeId,
    fmt::{self, Debug},
    ptr::{self, NonNull},
};

use ash::vk::Rect2D;

use super::{BuildContext, Text, Ui, UiEvent, ui::InputResult};
use crate::{
    primitives::Vec2,
    ui::{UiRef, widget::Widget},
};

pub struct UiElement {
    pub(crate) id: u32,
    pub name: &'static str,
    pub visible: bool,
    pub size: Vec2,
    pub pos: Vec2,
    pub z_index: f32,
    pub parent: Option<NonNull<Self>>,
    pub childs: Vec<Self>,
    pub widget: Box<dyn Widget>,
    pub type_id: TypeId,
}

impl UiElement {
    pub const fn id(&self) -> u32 {
        self.id
    }

    pub fn type_of<T: Widget>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    pub fn downcast<T: Widget>(&self) -> Option<&T> {
        if self.type_of::<T>() {
            let gg = unsafe { &*((&*self.widget) as *const dyn Widget as *const T) };
            Some(gg)
        } else {
            None
        }
    }

    pub fn downcast_mut<T: Widget>(&mut self) -> Option<&mut T> {
        if self.type_of::<T>() {
            let gg = unsafe { &mut *((&mut *self.widget) as *mut dyn Widget as *mut T) };
            Some(gg)
        } else {
            None
        }
    }

    pub(crate) fn childs_mut<'b>(&self) -> &'b mut Vec<Self> {
        unsafe {
            let childs: &Vec<Self> = &self.childs;
            #[allow(invalid_reference_casting)]
            &mut *(childs as *const Vec<Self> as *mut Vec<Self>)
        }
    }

    pub fn build(&mut self, context: &mut BuildContext) {
        context.z_index = self.z_index;
        context.element_pos = self.pos;
        context.element_size = self.size;

        let childs = self.childs_mut();

        self.widget.build(childs, context);

        self.pos = context.element_pos;
        self.size = context.element_size;
    }

    pub fn get_instances(&mut self, ui: &mut Ui, clip: Option<Rect2D>) {
        let mut inner_clip = clip;

        if self.visible {
            let element = unsafe { &*ptr::from_mut(self) };
            inner_clip = self.widget.instance(element, ui, clip);
        }

        for child in &mut self.childs {
            child.get_instances(ui, inner_clip);
        }
    }

    pub fn offset_element(&mut self, offset: Vec2) {
        self.pos += offset;

        if let Some(text) = self.downcast_mut::<Text>() {
            for i in &mut text.font_instances {
                i.pos += offset;
            }
            if let Some(cursor) = &mut text.cursor {
                cursor.pos += offset;
            }
        }

        for child in &mut self.childs {
            child.offset_element(offset);
        }
    }

    pub fn is_in(&self, pos: Vec2) -> bool {
        self.pos <= pos && self.pos.x + self.size.x >= pos.x && self.pos.y + self.size.y >= pos.y
    }

    pub fn get_text(&self) -> Option<&str> {
        self.get_text_at_pos(0)
    }

    pub fn get_child(&mut self, index: usize) -> Option<UiRef> {
        Some(UiRef::new(self.childs.get_mut(index)?))
    }

    pub fn get_text_at_pos(&self, pos: usize) -> Option<&str> {
        let child = self.childs.get(pos)?;
        if let Some(text) = child.downcast::<Text>() {
            Some(&text.text)
        } else {
            None
        }
    }

    pub fn handle_input(&mut self, ui: &mut Ui, event: UiEvent) -> InputResult {
        if !self.visible {
            return InputResult::None;
        }

        if self.is_in(ui.cursor_pos) {
            if !self.childs.iter().any(|c| c.id == ui.selection.hover_id()) {
                for child in &mut self.childs {
                    let result = child.handle_input(ui, event);
                    if !result.is_none() {
                        return result;
                    };
                }
            }
            let element = UiRef::new(self);
            let result = self.widget.interaction(element, ui, event);

            return result;
        }
        InputResult::None
    }

    /// Adds a child to self and returns a weak reference to it
    pub fn add_child(&mut self, child: UiElement) -> Option<UiRef> {
        let parent = Some(NonNull::from_mut(self));
        let childs = &mut self.childs;
        let ptr = childs.as_ptr();

        childs.push(child);

        // if a realloc happens we need to update the child pointers
        if ptr == childs.as_ptr() {
            for child in childs.iter_mut() {
                child.parent = parent;
            }

            Some(UiRef::new(childs.last_mut()?))
        // otherwise we only set the parent for the new node
        } else {
            let child = childs.last_mut()?;
            child.parent = parent;
            Some(UiRef::new(child))
        }
    }

    /// Removes all pointers that point to the element to prevent invalid dereferencing
    pub(crate) fn remove_residue(&self, ui: &mut Ui) {
        ui.remove_tick(self.id);
        ui.selection.check_removed(self.id);

        for child in &self.childs {
            child.remove_residue(ui);
        }
    }

    pub(crate) fn update_ptrs(&mut self, ui: &mut Ui) {
        ui.update_tick_ptrs(self);
        ui.selection.update_ptr(self);

        let element = NonNull::from_mut(self);

        for child in &mut self.childs {
            child.parent = Some(element);
        }
    }

    /// Sets id, parent and z-index for self and all childs
    pub(crate) fn init(&mut self, ui: &Ui) {
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
            } else if let Some(child) = child.get_child_by_id(id) {
                return Some(child);
            }
        }
        None
    }

    pub fn get_child_by_id_mut(&mut self, id: u32) -> Option<&mut UiElement> {
        for child in &mut self.childs {
            if child.id == id {
                return Some(child);
            } else if let Some(child) = child.get_child_by_id_mut(id) {
                return Some(child);
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
