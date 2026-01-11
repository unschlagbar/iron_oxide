use std::{
    any::TypeId,
    fmt::{self, Debug},
};

use ash::vk::Rect2D;
use winit::{event::KeyEvent, window::CursorIcon};

use super::{BuildContext, Text, Ui, UiEvent, system::InputResult};
use crate::{
    primitives::Vec2,
    ui::{UiRef, widget::Widget},
};
#[test]
fn size() {
    println!("size: {}", size_of::<UiElement>())
}

pub struct UiElement {
    pub(crate) id: u32,
    pub name: &'static str,
    pub visible: bool,
    pub transparent: bool,
    pub size: Vec2,
    pub pos: Vec2,
    pub z_index: i16,
    pub parent: Option<UiRef>,
    pub childs: Vec<Self>,
    pub(crate) widget: Box<dyn Widget>,
}

impl UiElement {
    pub const fn id(&self) -> u32 {
        self.id
    }

    pub fn type_of<T: Widget>(&self) -> bool {
        self.widget.type_id() == TypeId::of::<T>()
    }

    pub fn downcast<T: Widget>(&self) -> Option<&T> {
        if self.type_of::<T>() {
            let gg = unsafe { &*(&*self.widget as *const dyn Widget as *const T) };
            Some(gg)
        } else {
            None
        }
    }

    pub fn downcast_mut<T: Widget>(&mut self) -> Option<&mut T> {
        if self.type_of::<T>() {
            let gg = unsafe { &mut *(&mut *self.widget as *mut dyn Widget as *mut T) };
            Some(gg)
        } else {
            None
        }
    }

    fn childs_mut<'a>(&mut self) -> &'a mut Vec<Self> {
        unsafe {
            let childs = &mut self.childs;
            &mut *(childs as *mut Vec<Self>)
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
            let element = UiRef::new(self);
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

    pub fn get_text_at_pos(&self, pos: usize) -> Option<&str> {
        let child = self.childs.get(pos)?;
        if let Some(text) = child.downcast::<Text>() {
            Some(&text.text)
        } else {
            None
        }
    }

    pub fn update_hover(&mut self, ui: &mut Ui, _event: UiEvent) -> InputResult {
        if !self.visible {
            return InputResult::None;
        }

        if self.is_in(ui.cursor_pos) {
            for child in &mut self.childs {
                if child.update_hover(ui, _event) == InputResult::New {
                    return InputResult::New;
                }
            }

            if self.transparent {
                InputResult::None
            } else {
                ui.selection.set_hover(UiRef::new(self));
                InputResult::New
            }
        } else {
            InputResult::None
        }
    }

    pub fn handle_hover(&mut self, ui: &mut Ui, event: UiEvent) -> InputResult {
        let element = UiRef::new(self);

        let propagate = event != UiEvent::Move && event != UiEvent::HoverEnd;
        let result = self.widget.interaction(element, ui, event);

        if result == InputResult::New || !propagate {
            return InputResult::New;
        }

        if let Some(mut parent) = self.parent {
            unsafe { parent.as_mut().handle_hover(ui, event) }
        } else {
            InputResult::None
        }
    }

    pub fn handle_key(&mut self, ui: &mut Ui, event: &KeyEvent) -> InputResult {
        let element = UiRef::new(self);
        self.widget.key_event(element, ui, event)
    }

    /// Adds a child to self and returns a weak reference to it
    pub(crate) fn add_child(&mut self, child: UiElement, ui: &mut Ui) -> Option<UiRef> {
        let childs = &mut self.childs;
        let ptr = childs.as_ptr();

        childs.push(child);

        // if a realloc happens we need to update the child pointers
        if ptr != childs.as_ptr() {
            for child in childs.iter_mut() {
                child.update_ptrs(ui);
            }
        }
        Some(UiRef::new(childs.last_mut().unwrap()))
    }

    /// Removes all pointers that point to the element to prevent invalid dereferencing
    pub(crate) fn remove_residue(&self, ui: &mut Ui) {
        ui.remove_tick(self.id);
        if ui.selection.check_removed(self.id) {
            ui.cursor_icon = CursorIcon::Default;
        }

        for child in &self.childs {
            child.remove_residue(ui);
        }
    }

    pub(crate) fn update_ptrs(&mut self, ui: &mut Ui) {
        ui.update_tick_ptrs(UiRef::new(self));
        ui.selection.update_ptr(UiRef::new(self));

        let element = UiRef::new(self);

        for child in &mut self.childs {
            child.parent = Some(element);
        }
    }

    /// Sets id, parent and z-index for all childs
    pub(crate) fn init(&mut self, ui: &Ui) {
        let parent = Some(UiRef::new(self));
        let z_index = self.z_index + 10;
        for child in &mut self.childs {
            child.parent = parent;
            child.id = ui.get_id();
            child.z_index = z_index;
            child.init(ui);
        }
    }

    /// Returns a ref to the child
    /// Searches for childs with the id
    pub fn get_child(&self, id: u32) -> Option<UiRef> {
        for child in &self.childs {
            if child.id == id {
                return Some(UiRef::new_ref(child));
            } else if let Some(child) = child.get_child(id) {
                return Some(child);
            }
        }
        None
    }

    /// Returns the child at the index
    pub fn child(&self, idx: usize) -> Option<UiRef> {
        self.childs.get(idx).map(UiRef::new_ref)
    }

    pub fn get_child_mut(&mut self, id: u32) -> Option<&mut UiElement> {
        for child in &mut self.childs {
            if child.id == id {
                return Some(child);
            } else if let Some(child) = child.get_child_mut(id) {
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
            .field("parent", &self.parent)
            .finish()
    }
}
