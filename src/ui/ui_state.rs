use ash::vk;
use std::{
    ptr::{self, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};
use winit::dpi::PhysicalSize;

use super::{
    BuildContext, Font, UiElement, UiEvent,
    ui_element::{Element, TypeConst},
};
use crate::{
    graphics::{AtlasInstance, Buffer, FontInstance, TextureAtlas, UiInstance, VkBase},
    primitives::Vec2,
    ui::{
        ElementType,
        draw_data::{DrawData, InstanceData},
        ui_pipeline::Pipeline,
    },
};

#[derive(Debug)]
pub struct UiState {
    elements: Vec<UiElement>,
    pub selected: Selected,
    pub size: Vec2,
    pub cursor_pos: Vec2,
    pub font: Font,
    pub visible: bool,
    pub dirty: DirtyFlags,
    pub texts: Vec<FontInstance>,

    pub event: Option<QueuedEvent>,
    pub tick_queue: Vec<TickEvent>,
    pub elements_to_remove: Vec<(*mut UiElement, u32)>,

    pub texture_atlas: TextureAtlas,
    id_gen: AtomicU32,

    base_pipeline: Pipeline,
    font_pipeline: Pipeline,
    atlas_pipeline: Pipeline,

    instance_buffer: Buffer,
    font_instance_buffer: Buffer,

    draw_batches: Vec<(u32, u32, u32, Option<vk::Rect2D>)>, // (material_idx, offset, size, clip)
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
            selected: Selected::default(),
            texts: Vec::new(),

            event: None,
            tick_queue: Vec::new(),
            elements_to_remove: Vec::new(),

            font: Font::parse_from_bytes(include_bytes!("../../font/std1.fef")),
            texture_atlas: TextureAtlas::new((1024, 1024)),

            base_pipeline: Pipeline::null(),
            font_pipeline: Pipeline::null(),
            atlas_pipeline: Pipeline::null(),

            instance_buffer: Buffer::null(),
            font_instance_buffer: Buffer::null(),

            draw_batches: Vec::new(),
        }
    }

    pub fn add_element<T: Element + TypeConst>(&mut self, element: T) -> u32 {
        let id = self.get_id();
        let z_index = if matches!(T::ELEMENT_TYPE, ElementType::AbsoluteLayout) {
            0.5
        } else {
            0.01
        };
        let mut element = UiElement {
            id,
            typ: T::ELEMENT_TYPE,
            dirty: true,
            visible: true,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: null_mut(),
            element: Box::new(element),
            z_index,
        };

        element.init();
        self.elements.push(element);
        if T::DEFAULT_TICKING {
            let child = ptr::from_mut(self.elements.last_mut().unwrap());
            self.set_ticking(child);
        }
        self.dirty = DirtyFlags::Resize;
        id
    }

    pub fn add_child_to<T: Element + TypeConst>(&mut self, child: T, element: u32) -> Option<u32> {
        let id = self.get_id();
        let element = self.get_element(element)?;
        let mut child = UiElement {
            id,
            typ: T::ELEMENT_TYPE,
            dirty: true,
            visible: true,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: null_mut(),
            element: Box::new(child),
            z_index: element.z_index + 0.01,
        };

        child.init();
        let child = element.add_child(child);

        if T::DEFAULT_TICKING
            && let Some(child) = child
        {
            let child = ptr::from_mut(child);
            self.set_ticking(child);
        }

        self.dirty = DirtyFlags::Resize;
        Some(id)
    }

    pub fn remove_element(&mut self, parent: *mut UiElement, id: u32) -> Option<UiElement> {
        if parent.is_null() {
            if let Some(pos) = self.elements.iter().position(|c| c.id == id) {
                self.elements.remove(pos);
            } else {
                println!("Child to remove not found: {id}");
            }
        } else {
            let parent = unsafe { &mut *parent };
            if let Some(childs) = parent.element.childs_mut() {
                if let Some(pos) = childs.iter().position(|c| c.id == id) {
                    childs.remove(pos);
                } else {
                    println!("Child to remove not found: {id}");
                }
            }
        }
        None
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
        self.base_pipeline = Pipeline::create_ui::<UiInstance>(
            base,
            window_size,
            render_pass,
            descriptor,
            base_shaders,
        );
        self.font_pipeline = Pipeline::create_ui::<FontInstance>(
            base,
            window_size,
            render_pass,
            descriptor,
            font_shaders,
        );

        self.atlas_pipeline = Pipeline::create_ui::<AtlasInstance>(
            base,
            window_size,
            render_pass,
            descriptor,
            atlas_shaders,
        );

        self.texture_atlas
            .load_directory("C:/Dev/home_storage_vulkan/textures", base, cmd_pool);
    }

    pub fn build(&mut self) {
        self.selected.clear();

        let mut build_context = BuildContext::default(&self.font, self.size);

        for element in &mut self.elements {
            element.build(&mut build_context);
            build_context.order += 1;
        }
    }

    pub fn get_instaces(&mut self) -> DrawData {
        self.dirty = DirtyFlags::None;
        self.texts.clear();

        if !self.visible || self.elements.len() == 0 {
            return DrawData::default();
        }

        let mut instances = DrawData::default();
        let self_copy = unsafe { &mut *ptr::from_mut(self) };

        for raw_e in &mut self.elements {
            raw_e.get_instances(self_copy, &mut instances, None);
        }

        instances
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

    pub fn get_element_mut(&mut self, root: Vec<usize>) -> Option<&mut UiElement> {
        let mut h = self.elements.get_mut(*root.first()?)?;
        for i in 1..root.len() {
            if let Some(childs) = h.element.childs_mut() {
                h = childs.get_mut(*root.get(i)?)?;
            } else {
                return None;
            }
        }

        Some(h)
    }

    pub fn check_selected(&mut self, event: UiEvent) -> EventResult {
        let self_clone = unsafe { ptr::from_mut(self).as_mut().unwrap() };
        let mut result = EventResult::None;

        if !self.selected.is_none() {
            let element = unsafe { &mut *self.selected.ptr };
            let element2 = unsafe { &mut *self.selected.ptr };

            result = element.element.interaction(element2, self_clone, event);
        }
        result
    }

    pub fn update_cursor(&mut self, cursor_pos: Vec2, event: UiEvent) -> EventResult {
        let self_clone = unsafe { ptr::from_mut(self).as_mut().unwrap() };
        self.cursor_pos = cursor_pos;

        let mut result = self.check_selected(event);
        if !result.is_none() {
            return result;
        }

        for element in &mut self.elements {
            let r = element.update_cursor(self_clone, event);
            if !r.is_none() {
                result = r;
                break;
            }
        }

        result
    }

    pub fn resize(&mut self, new_size: Vec2) {
        self.dirty = DirtyFlags::Resize;
        self.size = new_size;
    }

    pub fn update(&mut self, base: &VkBase, command_pool: vk::CommandPool) {
        self.draw_batches.clear();

        if matches!(self.dirty, DirtyFlags::Resize) {
            self.build();
        }

        let ui_instances = self.get_instaces();

        let mut mat_index = 0;
        let mut data_buf = Vec::new();

        for draw in ui_instances.groups {
            let this_mat = draw.data.material_idx();
            if let InstanceData::Basic(data) = draw.data {
                if this_mat == mat_index {
                    self.draw_batches.push((
                        this_mat,
                        data_buf.len() as u32,
                        data.len() as u32,
                        draw.clip,
                    ));
                    data_buf.extend(data);
                } else {
                    println!("2. Mat");
                    mat_index = this_mat;

                    self.instance_buffer.destroy(&base.device);
                    self.instance_buffer = Buffer::device_local_slow(
                        &base,
                        command_pool,
                        &data_buf,
                        vk::BufferUsageFlags::VERTEX_BUFFER,
                    );
                    data_buf.clear();
                    self.draw_batches
                        .push((this_mat, 0, data.len() as u32, draw.clip));
                    data_buf.extend(data);
                }
            }
        }
        self.instance_buffer.destroy(&base.device);
        self.instance_buffer = Buffer::device_local_slow(
            &base,
            command_pool,
            &data_buf,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        );

        if !self.texts.is_empty() {
            self.font_instance_buffer.destroy(&base.device);

            self.font_instance_buffer = Buffer::device_local_slow(
                &base,
                command_pool,
                &self.texts,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
        }
    }

    pub fn set_event(&mut self, event: QueuedEvent) {
        self.event = Some(event);
    }

    fn mat_pipe(&self, mat_idx: u32) -> (&Pipeline, &Buffer) {
        match mat_idx {
            0 => (&self.base_pipeline, &self.instance_buffer),
            _ => unimplemented!(),
        }
    }

    pub fn draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            let mut last_mat = u32::MAX;
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.base_pipeline.layout,
                0,
                &[descriptor_set],
                &[],
            );
            for (mat, offset, size, clip) in &self.draw_batches {
                if last_mat != *mat {
                    last_mat = *mat;
                    let (pipeline, buffer) = self.mat_pipe(*mat);

                    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.this);
                    device.cmd_bind_vertex_buffers(cmd, 0, &[buffer.inner], &[0]);
                }
                if let Some(clip) = clip {
                    device.cmd_set_scissor(cmd, 0, &[*clip]);
                }
                device.cmd_draw(cmd, 4, *size, 0, *offset);
            }

            if !self.texts.is_empty() {
                device.cmd_bind_pipeline(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.font_pipeline.this,
                );
                device.cmd_bind_vertex_buffers(cmd, 0, &[self.font_instance_buffer.inner], &[0]);
                device.cmd_draw(
                    cmd,
                    4,
                    self.font_instance_buffer.size as u32 / size_of::<FontInstance>() as u32,
                    0,
                    0,
                );
            }
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            self.base_pipeline.destroy(device);
            self.font_pipeline.destroy(device);
            self.atlas_pipeline.destroy(device);

            device.free_memory(self.instance_buffer.mem, None);
            device.destroy_buffer(self.instance_buffer.inner, None);
            device.free_memory(self.font_instance_buffer.mem, None);
            device.destroy_buffer(self.font_instance_buffer.inner, None);
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
}

#[derive(Debug)]
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

#[repr(u8)]
#[derive(Debug, Default)]
pub enum SelectedFlags {
    #[default]
    Null,
    Selected,
    Pressed,
}

#[derive(Debug)]
pub struct Selected {
    pub ptr: *mut UiElement,
    pub selected: SelectedFlags,
}

impl Selected {
    pub const fn clear(&mut self) {
        self.ptr = null_mut();
        self.selected = SelectedFlags::Null;
    }

    pub const fn set_selected(&mut self, element: *mut UiElement) {
        self.ptr = element;
        self.selected = SelectedFlags::Selected;
    }

    pub const fn set_pressed(&mut self, element: *mut UiElement) {
        self.ptr = element;
        self.selected = SelectedFlags::Pressed;
    }

    pub const fn is_none(&self) -> bool {
        self.ptr.is_null()
    }

    pub const fn id(&self) -> u32 {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { (*self.ptr).id }
        }
    }

    pub const fn pressed(&self) -> bool {
        matches!(self.selected, SelectedFlags::Pressed)
    }
}

impl Default for Selected {
    fn default() -> Self {
        Self {
            ptr: null_mut(),
            selected: SelectedFlags::default(),
        }
    }
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
    pub event: UiEvent,
    pub message: u16,
}

impl QueuedEvent {
    pub fn new(element: &UiElement, event: UiEvent, message: u16) -> Self {
        Self {
            element_id: element.id,
            element_type: element.typ,
            event,
            message,
        }
    }
}
