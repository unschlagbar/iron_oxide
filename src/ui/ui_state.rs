use ash::vk;
use std::{
    ptr,
    sync::atomic::{AtomicU32, Ordering}, time::Instant,
};
use winit::dpi::PhysicalSize;

use super::{
    BuildContext, Font, UiElement, UiEvent,
    element::{Element, TypeConst},
};
use crate::{
    graphics::{TextureAtlas, VkBase},
    primitives::Vec2,
    ui::{
        ElementType,
        materials::{AtlasInstance, Basic, FontInstance, Material, UiInstance},
        selection::Selection,
    },
};

pub struct UiState {
    elements: Vec<UiElement>,
    pub size: Vec2,
    pub cursor_pos: Vec2,
    pub font: Font,
    pub visible: bool,
    pub dirty: DirtyFlags,

    // All this needs to be checke before element removal
    // If not checked this will result in undefined behavior!
    pub selection: Selection,
    pub event: Option<QueuedEvent>,
    pub tick_queue: Vec<TickEvent>,

    pub texture_atlas: TextureAtlas,
    id_gen: AtomicU32,

    pub materials: Vec<Box<dyn Material>>,
}

impl UiState {
    pub fn create(visible: bool) -> UiState {
        UiState {
            visible,
            elements: Vec::new(),
            dirty: DirtyFlags::Resize,
            size: Vec2::zero(),
            id_gen: AtomicU32::new(1),
            cursor_pos: Vec2::default(),

            selection: Selection::default(),
            event: None,
            tick_queue: Vec::new(),

            font: Font::parse_from_bytes(include_bytes!("../../font/std1.fef")),
            texture_atlas: TextureAtlas::new((1024, 1024)),

            materials: Vec::with_capacity(3),
        }
    }

    pub fn add_element<T: Element + TypeConst>(&mut self, element: T, name: &'static str) -> u32 {
        let self_2 = unsafe { &mut *(self as *mut Self) };

        let id = self.get_id();
        let z_index = if matches!(T::ELEMENT_TYPE, ElementType::AbsoluteLayout) {
            0.5
        } else {
            0.01
        };
        let mut element = UiElement {
            id,
            name,
            typ: T::ELEMENT_TYPE,
            visible: true,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            element: Box::new(element),
            z_index,
        };

        element.init();
        self.elements.push(element);

        let element = self.elements.last_mut().unwrap();

        if T::DEFAULT_TICKING {
            let child = ptr::from_mut(element);
            self_2.set_ticking(child);
        }

        if T::ELEMENT_TYPE == ElementType::AbsoluteLayout && element.is_in(self.cursor_pos) {
            self.selection.clear();
            self.update_cursor(self.cursor_pos, UiEvent::Move);
        }

        self.dirty = DirtyFlags::Resize;
        id
    }

    pub fn add_child_to<T: Element + TypeConst>(
        &mut self,
        child: T,
        name: &'static str,
        element: u32,
    ) -> Option<u32> {
        let id = self.get_id();
        let element = self.get_element(element)?;
        let mut child = UiElement {
            id,
            name,
            typ: T::ELEMENT_TYPE,
            visible: true,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: None,
            element: Box::new(child),
            z_index: element.z_index + 0.01,
        };

        child.init();
        let child = element.add_child(child);

        if let Some(child) = child {
            if T::DEFAULT_TICKING {
                let child = ptr::from_mut(child);
                self.set_ticking(child);
            }
        }

        self.dirty = DirtyFlags::Resize;
        Some(id)
    }

    pub fn remove_element(&mut self, element: &mut UiElement) -> Option<UiElement> {
        if let Some(mut parent) = element.parent {
            let parent_mut = unsafe { parent.as_mut() };

            if let Some(childs) = parent_mut.element.childs_mut() {
                if let Some(pos) = childs.iter().position(|c| c.id == element.id) {
                    element.remove_tick(self);
                    let out = Some(childs.remove(pos));

                    for child in &mut childs[pos..] {
                        child.set_parent(parent);
                    }

                    out
                } else {
                    println!("Child to remove not found: {}", element.id);
                    None
                }
            } else {
                None
            }
        } else {
            if let Some(pos) = self.elements.iter().position(|c| c.id == element.id) {
                element.remove_tick(self);
                Some(self.elements.remove(pos))
            } else {
                println!("Child to remove not found: {}", element.id);
                None
            }
        }
    }

    pub fn remove_element_by_id(&mut self, id: u32) -> Option<UiElement> {
        let self2 = unsafe { &mut *ptr::from_mut(self) };
        let element = self2.get_element(id)?;
        self.remove_element(element)
    }

    pub fn get_id(&self) -> u32 {
        self.id_gen.fetch_add(1, Ordering::Relaxed)
    }

    pub fn init_graphics(
        &mut self,
        base: &VkBase,
        cmd_pool: vk::CommandPool,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor: vk::DescriptorSetLayout,
        base_shaders: (&[u8], &[u8]),
        font_shaders: (&[u8], &[u8]),
        atlas_shaders: (&[u8], &[u8]),
    ) {
        self.size = window_size.into();
        self.materials.push(Basic::<UiInstance>::new(
            base,
            window_size,
            render_pass,
            descriptor,
            base_shaders,
        ));
        self.materials.push(Basic::<FontInstance>::new(
            base,
            window_size,
            render_pass,
            descriptor,
            font_shaders,
        ));

        self.materials.push(Basic::<AtlasInstance>::new(
            base,
            window_size,
            render_pass,
            descriptor,
            atlas_shaders,
        ));

        self.texture_atlas
            .load_directory("../home_storage_vulkan/textures", base, cmd_pool);
    }

    pub fn build(&mut self) {
        self.selection.clear();

        let mut build_context = BuildContext::default(&self.font, self.size);

        for element in &mut self.elements {
            element.build(&mut build_context);
        }
    }

    pub fn get_instaces(&mut self) {
        self.dirty = DirtyFlags::None;

        if !self.visible || self.elements.len() == 0 {
            return;
        }

        let self_copy = unsafe { &mut *ptr::from_mut(self) };

        for raw_e in &mut self.elements {
            raw_e.get_instances(self_copy, None);
        }
    }

    pub fn get_element(&mut self, id: u32) -> Option<&mut UiElement> {
        for element in &mut self.elements {
            if element.id == id {
                return Some(element);
            } else {
                let result = element.get_child_by_id(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn get_element_mut(&mut self, root: Vec<u32>) -> Option<&mut UiElement> {
        let mut h = self.elements.get_mut(*root.first()? as usize)?;
        for i in 1..root.len() {
            if let Some(childs) = h.element.childs_mut() {
                h = childs.get_mut(*root.get(i)? as usize)?;
            } else {
                return None;
            }
        }

        Some(h)
    }

    pub fn check_selected(&mut self, event: UiEvent) -> EventResult {
        let self2 = unsafe { &mut *ptr::from_mut(self) };
        self.selection.check(self2, event)
    }

    pub fn end_selection(&mut self) -> EventResult {
        let self2 = unsafe { ptr::from_mut(self).as_mut().unwrap() };

        self.cursor_pos = Vec2::new(f32::MAX, f32::MAX);

        self.selection.end(self2)
    }

    pub fn update_cursor(&mut self, cursor_pos: Vec2, event: UiEvent) -> EventResult {
        let self_clone = unsafe { ptr::from_mut(self).as_mut().unwrap() };
        self.cursor_pos = cursor_pos;

        let mut result = self.check_selected(event);

        for element in self.elements.iter_mut().rev() {
            if element.typ == ElementType::AbsoluteLayout
                && element.is_in(cursor_pos)
                && element.id != self.selection.hover_id()
            {
                self.selection.end(self_clone);
                let r = element.update_cursor(self_clone, event);
                if !r.is_none() {
                    result = r;
                }
                break;
            } else {
                let r = element.update_cursor(self_clone, event);
                if !r.is_none() {
                    result = r;
                    break;
                }
            }
        }

        result
    }

    pub fn get_hovered(&mut self) -> Option<&mut UiElement> {
        self.selection.get_hovered()
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        for mat in &mut self.materials {
            mat.destroy(device);
        }
        self.texture_atlas.destroy(device);
    }

    pub fn set_ticking(&mut self, element: *mut UiElement) {
        self.tick_queue.push(TickEvent::new(element));
    }

    pub fn process_ticks(&mut self) {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        let ui2 = unsafe { &mut *ptr::from_mut(self) };

        for tick in &self.tick_queue {
            if !tick.done {
                let id = tick.element_id;
                let element = if let Some(element) = ui.get_element(id) {
                    element
                } else {
                    println!("Tick element not found: {}", id);
                    continue;
                };
                let element2 = unsafe { &mut *ptr::from_mut(element) };

                element.element.tick(element2, ui2);
            } else {
                println!("Tick done: {}", tick.element_id);
            }
        }
        self.tick_queue.retain(|x| !x.done);
    }

    pub fn remove_tick(&mut self, id: u32) {
        if let Some(pos) = self.tick_queue.iter().position(|x| x.element_id == id) {
            self.tick_queue[pos].done = true;
        }
    }

    pub fn needs_ticking(&self) -> bool {
        !self.tick_queue.is_empty()
    }

    pub fn resize(&mut self, new_size: Vec2) {
        self.dirty = DirtyFlags::Resize;
        self.size = new_size;
    }

    pub fn set_event(&mut self, event: QueuedEvent) {
        self.event = Some(event);
    }

    pub fn update(&mut self, base: &VkBase, command_buffer: vk::CommandBuffer) {
        if !self.visible || matches!(self.dirty, DirtyFlags::None) {
            return;
        }

        let start = Instant::now();

        if matches!(self.dirty, DirtyFlags::Resize) {
            self.build();
        }

        for mat in &mut self.materials {
            mat.clear();
        }

        self.get_instaces();

        for mat in &mut self.materials {
            mat.update(base, command_buffer);
        }

        println!("time: {:?}", start.elapsed())
    }

    pub fn draw(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        descriptor_set: vk::DescriptorSet,
    ) {
        if !self.visible {
            return;
        }

        let clip = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: self.size.x as _,
                height: self.size.y as _,
            },
        };
        unsafe {
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.materials[0].pipeline().layout,
                0,
                &[descriptor_set],
                &[],
            );
            for mat in &self.materials {
                if mat.draw(device, cmd, clip) {
                    device.cmd_set_scissor(cmd, 0, &[clip]);
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EventResult {
    None,
    Old,
    New,
}

impl EventResult {
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub const fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }

    pub const fn is_old(&self) -> bool {
        matches!(self, Self::Old)
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum DirtyFlags {
    None,
    Resize,
    Color,
    Size,
}

#[derive(Debug)]
pub struct TickEvent {
    pub element_id: u32,
    pub done: bool,
    element: *mut UiElement,
}

impl TickEvent {
    pub fn new(element: *mut UiElement) -> Self {
        let element_id = unsafe { (*element).id };
        Self {
            element_id,
            done: false,
            element,
        }
    }

    pub fn element(&self) -> &mut UiElement {
        unsafe { &mut *self.element }
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    pub element_id: u32,
    pub element_type: ElementType,
    pub element_name: &'static str,
    pub event: UiEvent,
    pub message: u16,
}

impl QueuedEvent {
    pub fn new(element: &UiElement, event: UiEvent, message: u16) -> Self {
        Self {
            element_id: element.id,
            element_type: element.typ,
            element_name: element.name,
            event,
            message,
        }
    }
}
