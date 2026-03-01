use std::{
    any::TypeId,
    fmt::{self, Debug},
};

use ash::vk::Rect2D;
use winit::{event::KeyEvent, window::CursorIcon};

use super::{BuildContext, Text, Ui, UiEvent, system::InputResult};
use crate::{
    graphics::Ressources,
    primitives::Vec2,
    ui::{TextInput, UiRef, widget::Widget},
};

pub struct UiElement {
    pub(crate) id: u32,
    pub name: &'static str,
    pub flags: ElementFlags,
    pub size: Vec2<i16>,
    pub pos: Vec2<i16>,
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

    pub fn downcast_ref<T: Widget>(&self) -> Option<&T> {
        if self.type_of::<T>() {
            let gg = unsafe { &*(&*self.widget as *const dyn Widget as *const T) };
            Some(gg)
        } else {
            None
        }
    }

    pub fn downcast<T: Widget>(self) -> Option<T> {
        if self.type_of::<T>() {
            let raw = Box::into_raw(self.widget);
            let boxed_t = unsafe { Box::from_raw(raw as *mut T) };
            Some(*boxed_t)
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
        context.element_pos = Vec2::new(self.pos.x as f32, self.pos.y as f32);
        context.element_size = Vec2::new(self.size.x as f32, self.size.y as f32);

        let childs = self.childs_mut();
        self.widget.build_layout(childs, context);

        self.pos = Vec2::new(context.element_pos.x as i16, context.element_pos.y as i16);
    }

    pub fn build_size(&mut self, context: &mut BuildContext) {
        context.z_index = self.z_index;

        let childs = self.childs_mut();
        self.widget.build_size(childs, context);

        self.size = Vec2::new(context.element_size.x as i16, context.element_size.y as i16);
    }

    pub(crate) fn predict_size(&mut self, context: &mut BuildContext) {
        self.widget.predict_size(context);
    }

    pub fn get_draw_data(&mut self, ressources: &mut Ressources, info: DrawInfo) {
        let mut inner_info = info.inner(self.z_index);

        if self.flags.contains(ElementFlags::Visible) {
            let element = UiRef::new(self);
            self.widget.draw_data(element, ressources, &mut inner_info);
        }

        for child in &mut self.childs {
            child.get_draw_data(ressources, inner_info);
        }
    }

    pub fn offset_element(&mut self, offset: Vec2<f32>) {
        self.pos += Vec2::new(offset.x as i16, offset.y as i16);

        if let Some(text) = self.downcast_mut::<Text>() {
            text.offset += offset;
        } else if let Some(text) = self.downcast_mut::<TextInput>() {
            text.offset += offset;
        }

        for child in &mut self.childs {
            child.offset_element(offset);
        }
    }

    pub fn is_in(&self, pos: Vec2<i16>) -> bool {
        self.pos <= pos && self.pos.x + self.size.x >= pos.x && self.pos.y + self.size.y >= pos.y
    }

    pub fn get_text(&self) -> Option<&str> {
        self.get_text_at_pos(0)
    }

    pub fn get_text_at_pos(&self, pos: usize) -> Option<&str> {
        let child = self.childs.get(pos)?;
        if let Some(text) = child.downcast_ref::<Text>() {
            Some(&text.text)
        } else {
            None
        }
    }

    pub fn update_hover(&mut self, ui: &mut Ui, _event: UiEvent) -> InputResult {
        if !self.flags.contains(ElementFlags::Visible) {
            return InputResult::None;
        }

        if self.is_in(ui.cursor_pos) {
            for child in &mut self.childs {
                if child.update_hover(ui, _event) == InputResult::New {
                    return InputResult::New;
                }
            }

            if self.flags.contains(ElementFlags::Transparent) {
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

    /// Inserts a child to self and returns a weak reference to it
    pub(crate) fn insert_child(
        &mut self,
        child: UiElement,
        ui: &mut Ui,
        idx: usize,
    ) -> Option<UiRef> {
        let childs = &mut self.childs;
        let ptr = childs.as_ptr();

        childs.insert(idx, child);

        // if a realloc happens we need to update the child pointers
        if ptr != childs.as_ptr() {
            for child in childs.iter_mut() {
                child.update_ptrs(ui);
            }
        } else {
            for child in &mut childs[idx..] {
                child.update_ptrs(ui);
            }
        }

        childs.get_mut(idx).map(UiRef::new)
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
            .field("flags", &self.flags)
            .field("size", &self.size)
            .field("pos", &self.pos)
            .finish()
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ElementFlags: u8 {
        const Visible = 0b00000001;
        const Transparent = 0b00000010;
    }
}

impl Default for ElementFlags {
    fn default() -> Self {
        Self::Visible
    }
}

#[derive(Clone, Copy)]
pub struct DrawInfo {
    pub clip: Option<Rect2D>,
    pub scale_factor: f32,
    pub z_index: i16,
    pub z_start: i16,
    pub z_end: i16,
}

impl DrawInfo {
    pub fn inner(&self, z_index: i16) -> Self {
        Self {
            clip: self.clip,
            scale_factor: self.scale_factor,
            z_index,
            z_start: self.z_start,
            z_end: self.z_end,
        }
    }

    pub fn clip(&mut self, offset: Vec2<f32>, extend: Vec2<f32>) {
        self.clip = Some(Rect2D {
            offset: offset.into(),
            extent: extend.into(),
        });
    }
}
