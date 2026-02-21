use ash::vk;
use bitflags::bitflags;
use std::{
    ops::Range,
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};
use winit::window::CursorIcon;

use super::{BuildContext, Font, UiElement, UiEvent};
use crate::{
    graphics::{Ressources, VkBase},
    primitives::Vec2,
    ui::{
        Absolute, DrawInfo, QueuedEvent, Ticking, UiRef,
        events::{QueuedEventHandler, TickEvent},
        selection::{Select, Selection},
    },
};

pub struct Ui {
    pub size: Vec2<f32>,
    pub scale_factor: f32,
    pub cursor_pos: Vec2<i16>,

    /// Set of common modifiers for input events
    pub modifiers: KeyModifiers,

    /// Target Cursor.
    /// Should only be used from hovered Elements
    pub cursor_icon: CursorIcon,
    /// current Cursor.
    pub current_cursor_icon: CursorIcon,

    pub font: Font,
    pub visible: bool,
    pub(crate) dirty: DirtyFlags,
    pub new_absolute: bool,

    // Check all this before removing a Node!
    // If not checked this will result in undefined behavior!
    pub(crate) selection: Selection,
    pub events: QueuedEventHandler,
    pub tick_queue: Vec<TickEvent>,

    pub(crate) id_gen: AtomicU32,
    pub(crate) elements: Vec<UiElement>,
}

impl Ui {
    pub fn create(visible: bool) -> Self {
        Self {
            size: Vec2::zero(),
            scale_factor: 1.0,
            cursor_pos: Vec2::default(),

            modifiers: KeyModifiers::default(),

            font: Font::parse_from_bytes(include_bytes!("../../font/std2.fef")),
            visible,
            dirty: DirtyFlags::Layout,
            new_absolute: false,

            cursor_icon: CursorIcon::Default,
            current_cursor_icon: CursorIcon::Default,

            selection: Selection::default(),
            events: QueuedEventHandler::new(),
            tick_queue: Vec::new(),

            // 0 is reserved for invalid UiRef
            id_gen: AtomicU32::new(1),
            elements: Vec::new(),
        }
    }

    pub fn add_child_to_root(&mut self, mut element: UiElement) -> UiRef {
        // Todo Find better solution!
        let z_index = if element.type_of::<Absolute>() || element.type_of::<Ticking<Absolute>>() {
            self.new_absolute = true;
            500
        } else {
            10
        };
        let ticking = element.widget.is_ticking();

        element.id = self.get_id();
        element.z_index = z_index;
        element.parent = None;

        self.elements.push(element);
        let mut element = UiRef::new(self.elements.last_mut().unwrap());
        unsafe { element.as_mut().init(self) };

        if ticking {
            self.set_ticking(element);
        }

        self.layout_changed();
        element
    }

    fn element_to_ref<T: Into<Element>>(&mut self, element: T) -> Option<UiRef> {
        match element.into() {
            Element::Id(id) => self.get_element(id),
            Element::Ref(ui_ref) => Some(ui_ref),
        }
    }

    pub fn add_child<T: Into<Element>>(
        &mut self,
        mut child: UiElement,
        parent: T,
    ) -> Option<UiRef> {
        let mut parent = self.element_to_ref(parent)?;

        child.id = self.get_id();
        child.z_index = parent.z_index + 10;
        child.parent = Some(parent);

        let ticking = child.widget.is_ticking();
        let child = unsafe { parent.as_mut().add_child(child, self) };

        if let Some(mut child) = child {
            let child_mut = unsafe { child.as_mut() };
            child_mut.init(self);

            if ticking {
                self.set_ticking(child);
            }

            self.layout_changed();
            Some(child)
        } else {
            None
        }
    }

    pub fn insert_child<T: Into<Element>>(
        &mut self,
        mut child: UiElement,
        parent: T,
        idx: usize,
    ) -> Option<UiRef> {
        let mut parent = self.element_to_ref(parent)?;

        child.id = self.get_id();
        child.z_index = parent.z_index + 10;
        child.parent = Some(parent);

        let ticking = child.widget.is_ticking();
        let child = unsafe { parent.as_mut().insert_child(child, self, idx) };

        if let Some(mut child) = child {
            let child_mut = unsafe { child.as_mut() };
            child_mut.init(self);

            if ticking {
                self.set_ticking(child);
            }

            self.layout_changed();
            Some(child)
        } else {
            None
        }
    }

    pub fn remove_element<T: Into<Element>>(&mut self, element: T) -> Option<UiElement> {
        let element = self.element_to_ref(element)?;

        if let Some(mut parent) = element.parent {
            let parent = unsafe { parent.as_mut() };

            if let Some(i) = parent.childs.iter().position(|c| c.id == element.id) {
                element.remove_residue(self);
                self.layout_changed();
                let removed = parent.childs.remove(i);

                for shifted in &mut parent.childs[i..] {
                    shifted.update_ptrs(self);
                }
                Some(removed)
            } else {
                panic!("Child to remove not found: {}", element.id);
            }
        } else if let Some(i) = self.elements.iter().position(|c| c.id == element.id) {
            element.remove_residue(self);
            let removed = self.elements.remove(i);

            let elements = unsafe { &mut *(self.elements[i..].as_mut() as *mut [UiElement]) };

            for shifted in elements {
                shifted.update_ptrs(self);
            }
            Some(removed)
        } else {
            panic!("Child to remove not found: {}", element.id);
        }
    }

    pub fn remove_elements(&mut self, mut parent: UiRef, range: Range<usize>) {
        let i = range.start;
        let parent = unsafe { parent.as_mut() };

        for element in parent.childs.drain(range) {
            element.remove_residue(self);
        }

        // Not tested for safety
        for shifted in &mut parent.childs[i..] {
            shifted.update_ptrs(self);
        }
    }

    pub fn remove_all_elements(&mut self, parent: UiRef) {
        self.remove_elements(parent, 0..parent.childs.len())
    }

    pub fn remove_all(&mut self) {
        while !self.elements.is_empty() {
            let element = UiRef::new(self.elements.last_mut().unwrap());
            self.remove_element(element);
        }
    }

    pub(crate) fn get_id(&self) -> u32 {
        self.id_gen.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn build(&mut self) {
        let mut build_context = BuildContext::default(&self.font, self.size, self.scale_factor);

        for element in &mut self.elements {
            element.build_size(&mut build_context);
        }

        let mut build_context = BuildContext::default(&self.font, self.size, self.scale_factor);

        for element in &mut self.elements {
            element.build(&mut build_context);
        }
    }

    pub(crate) fn get_instaces(&mut self, ressources: &mut Ressources) {
        self.dirty = DirtyFlags::None;

        if !self.visible || self.elements.is_empty() {
            return;
        }

        let info = DrawInfo {
            clip: None,
            scale_factor: self.scale_factor,
            z_index: 0,
            z_start: 0,
            z_end: i16::MAX,
        };

        for raw_e in &mut self.elements {
            raw_e.get_draw_data(ressources, info);
        }
    }

    /// UiRef
    #[track_caller]
    pub fn get_element(&mut self, id: u32) -> Option<UiRef> {
        debug_assert_ne!(id, 0, "0 is reserved for invalid UiRef");
        debug_assert_ne!(
            id,
            u32::MAX,
            "The element has not initialized yet, so it has id of u32::MAX"
        );
        for element in &mut self.elements {
            if element.id == id {
                return Some(UiRef::new(element));
            } else if let Some(result) = element.get_child(id) {
                return Some(result);
            }
        }
        None
    }

    pub fn get_element_mut(&mut self, id: u32) -> Option<&mut UiElement> {
        for element in &mut self.elements {
            if element.id == id {
                return Some(element);
            } else {
                let result = element.get_child_mut(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn set_focus(&mut self, element: UiRef) {
        if let Some(input) = &mut self.selection.focused {
            let widget = &mut input.as_mut().widget;
            widget.interaction(input.as_ui_ref(), self, UiEvent::End);
        }
        self.selection.focused = Some(Select::new(element))
    }

    pub fn handle_input(&mut self, cursor_pos: Vec2<i16>, event: UiEvent) -> InputResult {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        self.cursor_pos = cursor_pos;

        // 1. Check and update Captured
        if let Some(captured) = &mut self.selection.captured {
            let element = UiRef::new(captured.as_mut());
            let widget = &mut captured.as_mut().widget;

            if event.is_release() {
                self.selection.captured = None;
            }

            if widget.interaction(element, self, event).is_new() {
                return InputResult::New;
            }
        } else if event == UiEvent::Press {
            let mut exit = false;
            if let Some(focused) = &mut self.selection.focused
                && !focused.as_ref().is_in(ui.cursor_pos)
            {
                let widget = &mut focused.as_mut().widget;
                widget.interaction(focused.as_ui_ref(), self, UiEvent::End);
                exit = true;
            }

            if exit {
                self.selection.focused = None
            }
        }

        // 2. Check for new hover
        let last_hovered = self.selection.hovered;

        if self.new_absolute || event == UiEvent::Move {
            for element in &mut self.elements {
                if element.is_in(ui.cursor_pos) {
                    // We still can't break since there could be a absolute element above
                    element.update_hover(ui, event);
                }
            }
            self.new_absolute = false;
        }

        if let Some(mut last) = last_hovered {
            if let Some(mut new) = self.selection.hovered
                && last != new
            {
                let widget = &mut last.as_mut().widget;
                widget.interaction(UiRef::new(last.as_mut()), self, UiEvent::HoverEnd);

                new.as_mut().handle_hover(ui, event)
            } else {
                last.as_mut().handle_hover(ui, event)
            }
        } else {
            InputResult::None
        }
    }

    pub fn get_hovered(&mut self) -> Option<&mut UiElement> {
        self.selection.get_hovered()
    }

    pub fn process_ticks(&mut self) {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        let ui2 = unsafe { &mut *ptr::from_mut(self) };

        self.tick_queue.retain(|x| !x.done);

        for tick in &self.tick_queue {
            let id = tick.element_id;

            if let Some(element) = ui.get_element_mut(id) {
                let element_ref = UiRef::new(element);
                element.widget.tick(element_ref, ui2);
            } else {
                panic!("Tick element not found: {}", id);
            };
        }
    }

    pub fn set_ticking(&mut self, element: UiRef) {
        self.tick_queue.push(TickEvent::new(element));
    }

    pub fn remove_tick(&mut self, id: u32) {
        if let Some(pos) = self.tick_queue.iter().position(|x| x.element_id == id) {
            self.tick_queue[pos].done = true;
        }
    }

    pub(crate) fn update_tick_ptrs(&mut self, element: UiRef) {
        if let Some(pos) = self
            .tick_queue
            .iter()
            .position(|x| x.element_id == element.id)
        {
            self.tick_queue[pos].element = element;
        }
    }

    pub fn needs_ticking(&self) -> bool {
        !self.tick_queue.is_empty()
    }

    pub fn resize(&mut self, new_size: Vec2<f32>) {
        self.layout_changed();
        self.size = new_size;
    }

    pub fn set_event(&mut self, event: QueuedEvent) {
        self.events.set(event);
    }

    pub fn get_event(&mut self) -> Option<QueuedEvent> {
        self.events.get()
    }

    pub fn color_changed(&mut self) {
        if !matches!(self.dirty, DirtyFlags::Layout) {
            self.dirty = DirtyFlags::Color;
        }
    }

    pub fn layout_changed(&mut self) {
        self.dirty = DirtyFlags::Layout;
    }

    pub const fn is_dirty(&self) -> bool {
        !matches!(self.dirty, DirtyFlags::None)
    }

    pub fn update(&mut self, base: &VkBase, ressources: &mut Ressources, start: usize) {
        if !self.visible || !self.is_dirty() {
            return;
        }

        if matches!(self.dirty, DirtyFlags::Layout) {
            self.build();
        }

        ressources.clear_batches();

        self.get_instaces(ressources);

        ressources.upload(base, start);
    }
}

//Vulkan & graphics Stuff!!
impl Ui {
    pub fn create_ubo_desc_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        };

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: 1,
            p_bindings: &layout_binding,
            ..Default::default()
        };

        unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }

    pub fn create_img_desc_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        };

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: 1,
            p_bindings: &layout_binding,
            ..Default::default()
        };

        unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InputResult {
    None,
    New,
}

impl InputResult {
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub const fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum DirtyFlags {
    None,
    Layout,
    Color,
}

pub enum Element {
    Id(u32),
    Ref(UiRef),
}

impl From<u32> for Element {
    fn from(value: u32) -> Self {
        Self::Id(value)
    }
}

impl From<UiRef> for Element {
    fn from(value: UiRef) -> Self {
        Self::Ref(value)
    }
}

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct KeyModifiers: u8 {
        const Shift = 0b00000001;
        const Strg = 0b00000010;
        const Alt = 0b00000100;
    }
}
