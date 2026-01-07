use ash::vk;
use std::{
    any::TypeId,
    ops::Range,
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
        Absolute, QueuedEvent, UiRef,
        events::TickEvent,
        materials::{AtlasInstance, Basic, FontInstance, Material, SingleImage, UiInstance},
        selection::{Select, Selection},
    },
};

pub const MAX_IMGS: u32 = 2;

pub struct Ui {
    pub(crate) elements: Vec<UiElement>,
    pub size: Vec2,
    pub cursor_pos: Vec2,
    pub mouse_down: bool,
    pub font: Font,
    pub visible: bool,
    pub(crate) dirty: DirtyFlags,
    pub different_dirty: bool,
    pub new_absolute: bool,

    // Check all this before removing a Node!
    // If not checked this will result in undefined behavior!
    pub(crate) selection: Selection,
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

impl Ui {
    pub fn create(visible: bool) -> Ui {
        Ui {
            visible,
            elements: Vec::new(),
            dirty: DirtyFlags::Layout,
            different_dirty: false,
            size: Vec2::zero(),
            id_gen: AtomicU32::new(1),
            cursor_pos: Vec2::default(),
            mouse_down: false,
            new_absolute: false,

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
        let z_index = if element.type_of::<Absolute>() {
            self.new_absolute = true;
            0.5
        } else {
            0.01
        };
        let ticking = element.widget.is_ticking();

        element.id = id;
        element.z_index = z_index;
        element.init(self);

        self.elements.push(element);
        let element = UiRef::new(self.elements.last_mut().unwrap());

        if ticking {
            self.set_ticking(&element);
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

        let ticking = child.widget.is_ticking();
        let child = element.get_mut(self).add_child(child);

        if let Some(child) = child {
            if ticking {
                self.set_ticking(&child);
            }

            self.layout_changed();
            Some(id)
        } else {
            None
        }
    }

    pub fn remove_element(&mut self, element: &UiElement) -> Option<UiElement> {
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
                println!("Child to remove not found: {}", element.id);
                None
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
            println!("Child to remove not found: {}", element.id);
            None
        }
    }

    pub fn remove_elements(&mut self, parent: UiRef, range: Range<usize>) {
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

    pub(crate) fn get_id(&self) -> u32 {
        self.id_gen.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn build(&mut self) {
        let mut build_context = BuildContext::default(&self.font, self.size);

        for element in &mut self.elements {
            element.build(&mut build_context);
        }
    }

    pub(crate) fn get_instaces(&mut self) {
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
        if let Some(input) = &mut self.selection.focused {
            let widget = &mut input.as_mut().widget;
            widget.interaction(UiRef::new_ref(element), self, UiEvent::End);
        }
        self.selection.focused = Some(Select::new(element))
    }

    /// TODO disable hover in touch mode
    pub fn handle_input(&mut self, cursor_pos: Vec2, event: UiEvent) -> InputResult {
        let ui = unsafe { &mut *ptr::from_mut(self) };
        self.cursor_pos = cursor_pos;

        // 1. Check and update Captured
        if let Some(captured) = &mut self.selection.captured {
            let widget = &mut captured.as_mut().widget;
            if widget.interaction(UiRef::new(captured.as_mut()), self, event) == InputResult::New {
                return InputResult::New;
            }
        }

        // 2. Check for new hover
        let last_hovered = self.selection.hovered;

        if self.new_absolute || event == UiEvent::Move {
            for element in &mut self.elements {
                if element.is_in(cursor_pos) {
                    // We still need to break since there could be a absolute element above
                    if element.type_of::<Absolute>() {
                        element.update_hover(ui, event);
                    } else {
                        element.update_hover(ui, event);
                    }
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

        self.tick_queue.retain(|x| !x.done);

        for tick in &self.tick_queue {
            let id = tick.element_id;

            if let Some(element) = ui.get_element_mut(id) {
                let element_ref = UiRef::new(element);
                element.widget.tick(element_ref, ui2);
            } else {
                println!("Tick element not found: {}", id);
            };
        }
    }

    pub fn set_ticking(&mut self, element: &UiElement) {
        self.tick_queue.push(TickEvent::new(element));
    }

    pub fn remove_tick(&mut self, id: u32) {
        if let Some(pos) = self.tick_queue.iter().position(|x| x.element_id == id) {
            self.tick_queue[pos].done = true;
        }
    }

    pub(crate) fn update_tick_ptrs(&mut self, element: &UiElement) {
        if let Some(pos) = self
            .tick_queue
            .iter()
            .position(|x| x.element_id == element.id)
        {
            self.tick_queue[pos].element = NonNull::from_ref(element);
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
impl Ui {
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
        if !self.visible || !self.is_dirty() {
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

        println!("CPU to GPU time: {:?}", start.elapsed())
    }

    pub fn draw(&mut self, device: &ash::Device, cmd: vk::CommandBuffer) {
        if !self.visible {
            return;
        }

        let clip = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.size.into(),
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
