use ash::vk;
use std::{
    any::TypeId,
    ptr::{self, NonNull},
    sync::atomic::{AtomicU32, Ordering},
    time::Instant,
};
use winit::dpi::PhysicalSize;

use super::{BuildContext, Font, UiElement, UiEvent};
use crate::{
    graphics::{Buffer, TextureAtlas, VkBase},
    primitives::{Matrix4, Vec2},
    ui::{
        Absolute, UiRef,
        materials::{AtlasInstance, Basic, FontInstance, Material, SingleImage, UiInstance},
        selection::Selection,
    },
};

pub const MAX_IMGS: u32 = 2;

pub struct UiState {
    pub(crate) elements: Vec<UiElement>,
    pub size: Vec2,
    pub cursor_pos: Vec2,
    pub font: Font,
    pub visible: bool,
    pub(crate) dirty: DirtyFlags,
    pub different_dirty: bool,
    pub ref_count: u16,

    // All this needs to be checke before element removal
    // If not checked this will result in undefined behavior!
    pub selection: Selection,
    pub event: Option<QueuedEvent>,
    pub tick_queue: Vec<TickEvent>,

    pub texture_atlas: TextureAtlas,
    pub(crate) id_gen: AtomicU32,

    pub(crate) desc_pool: vk::DescriptorPool,
    pub(crate) ubo_set: vk::DescriptorSet,
    pub(crate) img_set: vk::DescriptorSet,
    pub(crate) atl_set: vk::DescriptorSet,

    pub materials: Vec<Box<dyn Material>>,
    pub mat_table: Vec<TypeId>,
}

impl UiState {
    pub fn create(visible: bool) -> UiState {
        UiState {
            visible,
            elements: Vec::new(),
            dirty: DirtyFlags::Layout,
            different_dirty: false,
            size: Vec2::zero(),
            id_gen: AtomicU32::new(1),
            cursor_pos: Vec2::default(),
            ref_count: 0,

            selection: Selection::default(),
            event: None,
            tick_queue: Vec::new(),

            font: Font::parse_from_bytes(include_bytes!("../../font/std.fef")),
            texture_atlas: TextureAtlas::new((1024, 1024)),

            desc_pool: vk::DescriptorPool::null(),
            ubo_set: vk::DescriptorSet::null(),
            img_set: vk::DescriptorSet::null(),
            atl_set: vk::DescriptorSet::null(),

            materials: Vec::with_capacity(3),
            mat_table: Vec::with_capacity(3),
        }
    }

    pub fn add_child_to_root(&mut self, mut element: UiElement) -> u32 {
        let id = self.get_id();
        let z_index = if element.type_id == TypeId::of::<Absolute>() {
            0.5
        } else {
            0.01
        };
        let ticking = element.element.is_ticking();

        element.id = id;
        element.z_index = z_index;
        element.init(self);

        self.elements.push(element);
        let element = UiRef::new(self.elements.last_mut().unwrap());

        if ticking {
            self.set_ticking(&element);
        }

        if element.type_id == TypeId::of::<Absolute>() && element.is_in(self.cursor_pos) {
            self.selection.clear();
            self.update_cursor(self.cursor_pos, UiEvent::Move);
        }

        self.layout_changed();
        id
    }

    pub fn add_child_to(&mut self, mut child: UiElement, parent_id: u32) -> Option<u32> {
        let id = self.get_id();
        let element = self.get_element(parent_id)?;

        child.id = id;
        child.z_index = element.z_index + 0.01;

        child.init(self);
        let ticking = child.element.is_ticking();

        let child = element.get_mut(self).add_child(child);

        if let Some(child) = child
            && ticking
        {
            self.set_ticking(&child);
        }

        self.layout_changed();

        Some(id)
    }

    pub fn remove_element(&mut self, element: &UiElement) -> Option<UiElement> {
        if let Some(mut parent) = element.parent {
            let parent_mut = unsafe { parent.as_mut() };

            if true {
                if let Some(pos) = parent_mut.childs.iter().position(|c| c.id == element.id) {
                    self.remove_tick(element.id);
                    self.layout_changed();
                    Some(parent_mut.childs.remove(pos))
                } else {
                    println!("Child to remove not found: {}", element.id);
                    None
                }
            } else {
                None
            }
        } else if let Some(pos) = self.elements.iter().position(|c| c.id == element.id) {
            self.remove_tick(element.id);
            Some(self.elements.remove(pos))
        } else {
            println!("Child to remove not found: {}", element.id);
            None
        }
    }

    pub fn remove_element_by_id(&mut self, id: u32) -> Option<UiElement> {
        let element = self.get_element(id)?;
        self.remove_element(&element)
    }

    pub fn remove_all(&mut self) {
        while !self.elements.is_empty() {
            let element = UiRef::new(self.elements.last_mut().unwrap());
            self.remove_element(&element);
        }
    }

    pub fn get_id(&self) -> u32 {
        self.id_gen.fetch_add(1, Ordering::Relaxed)
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

        if !self.visible || self.elements.is_empty() {
            return;
        }

        let ui = unsafe { &mut *ptr::from_mut(self) };

        for raw_e in &mut self.elements {
            raw_e.get_instances(ui, None);
        }
    }

    /// UiRef
    pub fn get_element(&mut self, id: u32) -> Option<UiRef> {
        for element in &mut self.elements {
            if element.id == id {
                return Some(UiRef::new(element));
            } else {
                let result = element.get_child_by_id(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn get_element_mut(&mut self, id: u32) -> Option<&mut UiElement> {
        for element in &mut self.elements {
            if element.id == id {
                return Some(element);
            } else {
                let result = element.get_child_by_id_mut(id);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn set_focus(&mut self, element: &UiElement) {
        if let Some(mut input) = self.selection.active_input {
            println!("fire");
            unsafe { input.as_mut().element.interaction(UiRef::new_ref(element), self, UiEvent::End) };
        }
        self.selection.active_input = Some(NonNull::from_ref(element))
    }

    pub fn check_selected(&mut self, event: UiEvent) -> EventResult {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        self.selection.check(ui, event)
    }

    pub fn update_cursor(&mut self, cursor_pos: Vec2, event: UiEvent) -> EventResult {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        self.cursor_pos = cursor_pos;

        let mut result = self.check_selected(event);

        for element in self.elements.iter_mut() {
            if let Some(_) = element.downcast::<Absolute>()
                && element.is_in(cursor_pos)
            {
                self.selection.end(ui);
                let r = element.update_cursor(ui, event);
                if !r.is_none() {
                    result = r;
                }
                break;
            } else {
                let r = element.update_cursor(ui, event);
                if !r.is_none() {
                    result = r;
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
        unsafe {
            device.destroy_descriptor_pool(self.desc_pool, None);
        };
    }

    pub fn process_ticks(&mut self) {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        let ui2 = unsafe { &mut *ptr::from_mut(self) };

        for tick in &self.tick_queue {
            if !tick.done {
                let id = tick.element_id;
                let element = if let Some(element) = ui.get_element_mut(id) {
                    element
                } else {
                    println!("Tick element not found: {}", id);
                    continue;
                };

                let element_ref = UiRef::new(element);

                element.element.tick(element_ref, ui2);
            } else {
                println!("Tick done: {}", tick.element_id);
            }
        }
        self.tick_queue.retain(|x| !x.done);
    }

    pub fn set_ticking(&mut self, element: &UiElement) {
        self.tick_queue.push(TickEvent::new(element));
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
        self.layout_changed();
        self.size = new_size;
    }

    pub fn set_event(&mut self, event: QueuedEvent) {
        self.event = Some(event);
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
}

//Vulkan & graphics Stuff!!
impl UiState {
    #[allow(clippy::too_many_arguments)]
    pub fn init_graphics(
        &mut self,
        base: &VkBase,
        texture_atlas: TextureAtlas,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        uniform_buffer: &Buffer,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        base_shaders: (&[u8], &[u8]),
        font_shaders: (&[u8], &[u8]),
        atlas_shaders: (&[u8], &[u8]),
    ) {
        self.size = window_size.into();
        let ubo_layout = Self::create_ubo_desc_layout(&base.device);
        let img_layout = Self::create_img_desc_layout(&base.device);

        self.texture_atlas = texture_atlas;
        let atlas_view = self.texture_atlas.atlas.as_ref().unwrap().view;

        self.create_desc_pool(&base.device);
        self.create_desc_sets(
            &base.device,
            &[ubo_layout, img_layout, img_layout],
            uniform_buffer,
            image_view,
            atlas_view,
            sampler,
        );

        self.add_mat(Basic::<UiInstance>::new(
            base,
            window_size,
            render_pass,
            &[ubo_layout],
            base_shaders,
        ));

        self.add_mat(SingleImage::<FontInstance>::new(
            base,
            window_size,
            render_pass,
            &[ubo_layout, img_layout],
            self.img_set,
            font_shaders,
        ));

        self.add_mat(SingleImage::<AtlasInstance>::new(
            base,
            window_size,
            render_pass,
            &[ubo_layout, img_layout],
            self.atl_set,
            atlas_shaders,
        ));

        unsafe {
            base.device.destroy_descriptor_set_layout(ubo_layout, None);
            base.device.destroy_descriptor_set_layout(img_layout, None);
        }
    }

    fn add_mat<T: Material + 'static>(&mut self, material: T) {
        self.materials.push(Box::new(material));
        self.mat_table.push(TypeId::of::<T>());
    }

    pub fn update(&mut self, base: &VkBase, command_buffer: vk::CommandBuffer) {
        if !self.visible || matches!(self.dirty, DirtyFlags::None) {
            return;
        }

        let start = Instant::now();

        if matches!(self.dirty, DirtyFlags::Layout) {
            self.build();
        }

        for mat in &mut self.materials {
            mat.clear();
        }

        self.get_instaces();

        for mat in &mut self.materials {
            mat.update(base, command_buffer);
        }

        let memory_barrier = vk::MemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            ..Default::default()
        };

        unsafe {
            base.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[memory_barrier],
                &[],
                &[],
            );
        }

        println!("time: {:?}", start.elapsed())
    }

    pub fn draw(&mut self, device: &ash::Device, cmd: vk::CommandBuffer) {
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
                &[self.ubo_set],
                &[],
            );
            for mat in &self.materials {
                if mat.draw(device, cmd, clip) {
                    device.cmd_set_scissor(cmd, 0, &[clip]);
                }
            }
        }
    }

    pub fn create_desc_pool(&mut self, device: &ash::Device) {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_IMGS,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as _,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_IMGS + 1,
            ..Default::default()
        };

        self.desc_pool = unsafe { device.create_descriptor_pool(&pool_info, None).unwrap() }
    }

    fn create_ubo_desc_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
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

    fn create_img_desc_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
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

    fn create_desc_sets(
        &mut self,
        device: &ash::Device,
        layouts: &[vk::DescriptorSetLayout],
        uniform_buffer: &Buffer,
        image_view: vk::ImageView,
        atlas_view: vk::ImageView,
        sampler: vk::Sampler,
    ) {
        let allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: self.desc_pool,
            descriptor_set_count: layouts.len() as _,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let mut sets = unsafe {
            device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()
                .into_iter()
        };

        let ubo_set = sets.next().unwrap();
        let img_set = sets.next().unwrap();
        let atl_set = sets.next().unwrap();

        let buffer_info = vk::DescriptorBufferInfo {
            buffer: uniform_buffer.inner,
            offset: 0,
            range: size_of::<Matrix4>() as u64,
        };

        let image_info = vk::DescriptorImageInfo {
            sampler,
            image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let atlas_image_info = vk::DescriptorImageInfo {
            sampler,
            image_view: atlas_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let descriptor_writes = [
            vk::WriteDescriptorSet {
                dst_set: ubo_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: img_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &image_info,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: atl_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &atlas_image_info,
                ..Default::default()
            },
        ];

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };

        self.ubo_set = ubo_set;
        self.img_set = img_set;
        self.atl_set = atl_set;
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
#[derive(Debug, Clone, Copy)]
pub enum DirtyFlags {
    None,
    Layout,
    Color,
}

#[derive(Debug)]
pub struct TickEvent {
    pub element_id: u32,
    pub done: bool,
    pub element: NonNull<UiElement>,
}

impl TickEvent {
    pub fn new(element: &UiElement) -> Self {
        let element_id = element.id;
        let element = NonNull::from_ref(element);
        Self {
            element_id,
            done: false,
            element,
        }
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    pub element_id: u32,
    pub element_name: &'static str,
    pub event: UiEvent,
    pub message: u16,
}

impl QueuedEvent {
    pub fn new(element: &UiElement, event: UiEvent, message: u16) -> Self {
        Self {
            element_id: element.id,
            element_name: element.name,
            event,
            message,
        }
    }
}
