use ash::vk;
use std::{
    ptr::null_mut,
    sync::atomic::{AtomicU32, Ordering},
};
use winit::dpi::PhysicalSize;

use super::{
    BuildContext, Font, UiElement,
    raw_ui_element::UiEvent,
    ui_element::{Element, TypeConst},
    ui_pipeline,
};
use crate::{
    graphics::{Buffer, FontInstance, UiInstance, VkBase},
    primitives::Vec2,
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
    id_gen: AtomicU32,

    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    instance_buffer: Buffer,

    font_pipeline_layout: vk::PipelineLayout,
    font_pipeline: vk::Pipeline,
    font_instance_buffer: Buffer,
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
            font: Font::parse_from_bytes(include_bytes!("../../font/std1.fef")),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            instance_buffer: Buffer::null(),

            font_pipeline_layout: vk::PipelineLayout::null(),
            font_pipeline: vk::Pipeline::null(),
            font_instance_buffer: Buffer::null(),
        }
    }

    pub fn add_element<T: Element + TypeConst + 'static>(&mut self, element: T) {
        let mut element = UiElement {
            id: self.get_id(),
            typ: T::ELEMENT_TYPE,
            dirty: true,
            visible: true,
            size: Vec2::default(),
            pos: Vec2::default(),
            parent: null_mut(),
            element: Box::new(element),
            z_index: 0.01,
        };


        element.init();
        self.elements.push(element);
    }

    pub fn get_id(&self) -> u32 {
        self.id_gen.fetch_add(1, Ordering::Relaxed)
    }

    pub fn init_graphics(
        &mut self,
        base: &VkBase,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor: vk::DescriptorSetLayout,
        shaders: (&[u8], &[u8]),
        font_shader: (&[u8], &[u8]),
    ) {
        self.size = window_size.into();
        (self.pipeline_layout, self.pipeline) =
            ui_pipeline::basic_ui_pipeline(base, window_size, render_pass, descriptor, shaders);
        (self.font_pipeline_layout, self.font_pipeline) = super::font_pipeline::font_pipeline(
            base,
            window_size,
            render_pass,
            descriptor,
            font_shader,
        )
    }

    pub fn build(&mut self) {
        self.selected.clear();

        let mut build_context = BuildContext::default(&self.font, self.size);

        for element in &mut self.elements {
            element.build(&mut build_context);
            build_context.order += 1;
        }
    }

    pub fn get_instaces(&mut self) -> Vec<UiInstance> {
        self.dirty = DirtyFlags::None;
        self.texts.clear();

        let mut instances = Vec::new();
        let self_copy = unsafe { &mut *(self as *mut UiState) };

        if !self.visible || self.elements.len() == 0 {
            instances.push(UiInstance::default());
            return instances;
        }

        for raw_e in &mut self.elements {
            raw_e.get_instances(self_copy, &mut instances);
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
            h = h.element.childs().get_mut(*root.get(i)?)?;
        }

        Some(h)
    }

    pub fn update_cursor(&mut self, cursor_pos: Vec2, event: UiEvent) -> EventResult {
        //0 = no event
        //1 = no event break
        //2 = old event
        //3 = new event

        let self_clone = unsafe { &mut *(self as *mut UiState) };
        let mut result = EventResult::None;

        if !self.selected.is_none() {
            let element = unsafe { &mut *self.selected.ptr };
            let element2 = unsafe { &mut *self.selected.ptr };
            let element_result = element
                .element
                .interaction(element2, self_clone, cursor_pos, event);
            if !element_result.is_none() {
                return element_result;
            }
        }

        for element in &mut self.elements {
            let r = element.update_cursor(self_clone, cursor_pos, event);
            if !r.is_none() {
                result = r;
                break;
            }
        }

        self.cursor_pos = cursor_pos;
        result
    }

    pub fn resize(&mut self, new_size: Vec2) {
        self.dirty = DirtyFlags::Resize;
        self.size = new_size;
    }

    pub fn update(&mut self, base: &VkBase, command_pool: vk::CommandPool) {
        if matches!(self.dirty, DirtyFlags::Resize) {
            self.dirty = DirtyFlags::None;
            self.build();
            let ui_instances = self.get_instaces();
            if ui_instances.is_empty() {
                return;
            }

            unsafe { base.device.queue_wait_idle(base.queue).unwrap() };
            self.instance_buffer.destroy(&base.device);
            self.font_instance_buffer.destroy(&base.device);

            self.font_instance_buffer = Buffer::device_local_slow(
                &base,
                command_pool,
                &self.texts,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
            self.instance_buffer = Buffer::device_local_slow(
                &base,
                command_pool,
                &ui_instances,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
        } else if matches!(self.dirty, DirtyFlags::Color | DirtyFlags::Size) {
            let ui_instances = self.get_instaces();

            self.instance_buffer.destroy(&base.device);
            self.instance_buffer = Buffer::device_local_slow(
                &base,
                command_pool,
                &ui_instances,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );

            self.font_instance_buffer.destroy(&base.device);
            self.font_instance_buffer = Buffer::device_local_slow(
                &base,
                command_pool,
                &self.texts,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
        }
    }

    pub fn draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.instance_buffer.inner], &[0]);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            device.cmd_draw(
                cmd,
                4,
                self.instance_buffer.size as u32 / size_of::<UiInstance>() as u32,
                0,
                0,
            );

            if !self.texts.is_empty() {
                device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.font_pipeline);
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
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.free_memory(self.instance_buffer.mem, None);
            device.destroy_buffer(self.instance_buffer.inner, None);
            device.destroy_pipeline(self.font_pipeline, None);
            device.destroy_pipeline_layout(self.font_pipeline_layout, None);
            device.free_memory(self.font_instance_buffer.mem, None);
            device.destroy_buffer(self.font_instance_buffer.inner, None);
        }
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
        matches!(self, EventResult::None)
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
