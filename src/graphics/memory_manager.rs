use std::ptr;

use ash::vk::{
    Buffer, BufferCreateInfo, BufferUsageFlags, CommandPool, DeviceMemory, Image, ImageLayout,
    MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags,
};

use crate::graphics::{SinlgeTimeCommands, VkBase, image::VulkanImage};

#[derive(Debug)]
pub struct MemManager {
    pub is_uma: bool,
    pub host_visible: u32,
    pub device_local: u32,
    pub lazy: u32,

    pub memory_pool: [Memory; 5],
    pub buffers: Vec<ManagedBuffer>,
    pub images: Vec<ManagedImage>,
}

impl MemManager {
    pub fn new(base: &VkBase) -> Self {
        let mem_properties = unsafe {
            base.instance
                .get_physical_device_memory_properties(base.physical_device)
        };

        let mut host_visible = u32::MAX;
        let mut device_local = u32::MAX;
        let mut lazy = u32::MAX;

        for (i, mem) in mem_properties.memory_types_as_slice().iter().enumerate() {
            if device_local == u32::MAX
                && mem
                    .property_flags
                    .contains(MemoryPropertyFlags::DEVICE_LOCAL)
            {
                device_local = i as u32;
            }

            if host_visible == u32::MAX
                && mem.property_flags.contains(
                    MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
                )
            {
                host_visible = i as u32;
            }

            if lazy == u32::MAX
                && mem.property_flags.contains(
                    MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::LAZILY_ALLOCATED,
                )
            {
                lazy = i as u32;
            }
        }

        if host_visible == u32::MAX || device_local == u32::MAX {
            panic!("Missing memory Types");
        }

        if lazy == u32::MAX {
            lazy = device_local;
        } else {
            println!("MemoryPropertyFlags::LAZILY_ALLOCATED is supported");
        }

        let is_uma = host_visible == device_local;

        Self {
            is_uma,
            host_visible,
            device_local,
            lazy,
            memory_pool: [Memory::default(); 5],
            buffers: Vec::new(),
            images: Vec::new(),
        }
    }

    pub fn allocate_memory(
        &mut self,
        base: &VkBase,
        memory_type_index: u32,
        allocation_size: u64,
        slot: usize,
    ) {
        debug_assert_eq!(self.memory_pool[slot].memory, DeviceMemory::null());

        let alloc_info = MemoryAllocateInfo {
            allocation_size,
            memory_type_index,
            ..Default::default()
        };

        let mem = unsafe { base.device.allocate_memory(&alloc_info, None).unwrap() };

        self.memory_pool[slot] = Memory {
            mem_type_idx: memory_type_index,
            size: allocation_size,
            used: 0,
            memory: mem,
            mapped: ptr::null_mut(),
        };
    }

    pub fn reallocate_memory(&mut self, base: &VkBase, allocation_size: u64, slot: usize) {
        let mem = &mut self.memory_pool[slot];
        debug_assert_ne!(mem.memory, DeviceMemory::null());

        unsafe { base.device.free_memory(mem.memory, None) };
        mem.memory = DeviceMemory::null();

        let memory_type_index = mem.mem_type_idx;
        self.allocate_memory(base, memory_type_index, allocation_size, slot);
    }

    pub fn map_memory(&mut self, base: &VkBase, slot: usize, offset: u64, size: u64) -> *mut u8 {
        let memory = &mut self.memory_pool[slot];
        debug_assert_ne!(memory.memory, DeviceMemory::null());
        debug_assert_eq!(memory.mapped, ptr::null_mut());

        let size = if size == u64::MAX { memory.size } else { size };

        let mapped = unsafe {
            base.device
                .map_memory(memory.memory, offset, size, MemoryMapFlags::empty())
                .unwrap() as *mut u8
        };
        memory.mapped = mapped;
        mapped
    }

    pub fn unmap_memory(&mut self, base: &VkBase, slot: usize) {
        let memory = &mut self.memory_pool[slot];
        debug_assert_ne!(memory.memory, DeviceMemory::null());
        debug_assert_ne!(memory.mapped, ptr::null_mut());

        unsafe { base.device.unmap_memory(memory.memory) };
    }

    pub fn create_buffer(
        &mut self,
        base: &VkBase,
        mem_slot: usize,
        size: u64,
        usage: BufferUsageFlags,
    ) -> (Buffer, u64) {
        debug_assert_ne!(size, 0, "Size must be larger than 0");

        let buffer_info = BufferCreateInfo {
            size,
            usage,
            ..Default::default()
        };

        let buffer = unsafe { base.device.create_buffer(&buffer_info, None).unwrap() };
        let requirements = unsafe { base.device.get_buffer_memory_requirements(buffer) };

        let align = requirements.alignment;
        let buffer_size = (requirements.size + align - 1) & !(align - 1);

        let mem = &mut self.memory_pool[mem_slot];
        debug_assert_ne!(mem.memory, DeviceMemory::null());
        debug_assert!(mem.correct_mem_type(requirements.memory_type_bits));

        let offset = mem.bind_buffer(base, buffer, buffer_size, align);

        self.buffers
            .push(ManagedBuffer::new(buffer, mem_slot, offset, buffer_size));
        (buffer, buffer_size)
    }

    pub fn destroy_buffers(&mut self, base: &VkBase, start: usize) {
        if start > 0 {
            let buffer = &self.buffers[start - 1];
            self.memory_pool[0].used = buffer.offset + buffer.size;
        } else {
            self.memory_pool[0].used = 0;
        }
        unsafe {
            self.buffers
                .drain(start..)
                .for_each(|b| base.device.destroy_buffer(b.buffer, None));
        }
    }

    pub fn destroy_images(&mut self, base: &VkBase, start: usize) {
        if start > 0 {
            let image = &self.images[start - 1];
            self.memory_pool[0].used = image.offset + image.size;
        } else {
            self.memory_pool[0].used = 0;
        }
        unsafe {
            self.images
                .drain(start..)
                .for_each(|i| base.device.destroy_image(i.image, None));
        }
    }

    pub fn upload_image(
        &mut self,
        base: &VkBase,
        mem_slot: usize,
        cmd_pool: CommandPool,
        image: &mut VulkanImage,
        layout: ImageLayout,
        data: &[u8],
    ) {
        let requirements = unsafe { base.device.get_image_memory_requirements(image.image) };
        debug_assert_ne!(requirements.size, 0, "Size must be larger than 0");

        let (buffer, _) =
            self.create_buffer(base, 0, requirements.size, BufferUsageFlags::TRANSFER_SRC);

        let mem = self.memory_pool[0].as_ptr(self.buffers.last().unwrap());
        unsafe {
            mem.copy_from_nonoverlapping(data.as_ptr(), data.len());
        };

        let align = requirements.alignment;
        let size = (requirements.size + align - 1) & !(align - 1);

        let mem = &mut self.memory_pool[mem_slot];
        debug_assert_ne!(mem.memory, DeviceMemory::null());
        debug_assert!(mem.correct_mem_type(requirements.memory_type_bits));

        let offset = mem.bind_image(base, image.image, size, align);

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);

        image.trasition_layout(base, cmd_buf, ImageLayout::TRANSFER_DST_OPTIMAL);
        image.copy_from_buffer(base, cmd_buf, buffer);
        image.trasition_layout(base, cmd_buf, layout);

        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);
        self.pop_buffer(base);

        self.images
            .push(ManagedImage::new(image.image, mem_slot, offset, size));
    }

    pub fn create_image(
        &mut self,
        base: &VkBase,
        mem_slot: usize,
        cmd_pool: CommandPool,
        image: &mut VulkanImage,
        layout: ImageLayout,
    ) {
        let requirements = unsafe { base.device.get_image_memory_requirements(image.image) };
        debug_assert_ne!(requirements.size, 0, "Size must be larger than 0");

        let align = requirements.alignment;
        let size = (requirements.size + align - 1) & !(align - 1);

        let mem = &mut self.memory_pool[mem_slot];
        debug_assert_ne!(mem.memory, DeviceMemory::null());
        debug_assert!(mem.correct_mem_type(requirements.memory_type_bits));

        mem.bind_image(base, image.image, size, align);

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);
        image.trasition_layout(base, cmd_buf, layout);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);
    }

    pub fn destroy_buffer(&mut self, base: &VkBase, buffer: Buffer) {
        if let Some(pos) = self.buffers.iter_mut().position(|b| b.buffer == buffer) {
            unsafe { base.device.destroy_buffer(self.buffers[pos].buffer, None) };

            if pos + 1 == self.buffers.len() {
                let buffer = self.buffers.pop().unwrap();
                self.memory_pool[0].used = buffer.offset;
            } else if self.buffers[pos..]
                .iter()
                .all(|b| b.buffer == Buffer::null())
            {
                self.memory_pool[0].used = self.buffers[pos].offset;
                self.buffers.drain(pos..);
            } else {
                self.buffers[pos].buffer = Buffer::null();
            }
        }
    }

    pub fn pop_buffer(&mut self, base: &VkBase) {
        let buffer = self.buffers.pop().unwrap();
        unsafe { base.device.destroy_buffer(buffer.buffer, None) };

        self.memory_pool[buffer.mem_slot].used = buffer.offset;
    }

    pub fn destroy_image(&mut self, base: &VkBase, image: Image) {
        if let Some(pos) = self.images.iter_mut().position(|i| i.image == image) {
            let managed = &self.images[pos];
            unsafe { base.device.destroy_image(managed.image, None) };

            if pos + 1 == self.images.len() {
                let image = self.images.pop().unwrap();
                self.memory_pool[image.mem_slot].used = image.offset;
            } else if self.images[pos..].iter().all(|i| i.image == Image::null()) {
                self.memory_pool[managed.mem_slot].used = self.images[pos].offset;
                self.images.drain(pos..);
            } else {
                self.images[pos].image = Image::null();
            }
        }
    }

    pub fn destroy(&mut self, base: &VkBase) {
        self.destroy_buffers(base, 0);
        self.destroy_images(base, 0);

        for mem in self.memory_pool {
            if mem.memory != DeviceMemory::null() {
                mem.destroy(base)
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Memory {
    pub mem_type_idx: u32,
    pub size: u64,
    pub used: u64,
    pub memory: DeviceMemory,
    pub mapped: *mut u8,
}

impl Memory {
    pub fn destroy(self, base: &VkBase) {
        unsafe {
            base.device.free_memory(self.memory, None);
        }
    }

    pub fn bind_buffer(&mut self, base: &VkBase, buffer: Buffer, size: u64, align: u64) -> u64 {
        let offset = (self.used + align - 1) & !(align - 1);
        unsafe {
            base.device
                .bind_buffer_memory(buffer, self.memory, offset)
                .unwrap()
        };
        self.used = offset + size;
        offset
    }

    pub fn bind_image(&mut self, base: &VkBase, image: Image, size: u64, align: u64) -> u64 {
        let offset = (self.used + align - 1) & !(align - 1);
        unsafe {
            base.device
                .bind_image_memory(image, self.memory, offset)
                .unwrap()
        };
        self.used = offset + size;
        offset
    }

    pub fn correct_mem_type(&self, memory_type_bits: u32) -> bool {
        memory_type_bits & (1 << self.mem_type_idx) != 0
    }

    pub fn get_ptr(&self, offset: usize) -> *mut u8 {
        debug_assert_ne!(self.mapped, ptr::null_mut());
        unsafe { self.mapped.add(offset) }
    }

    #[inline]
    pub fn as_ptr(&self, buffer: &ManagedBuffer) -> *mut u8 {
        self.get_ptr(buffer.offset as usize)
    }
}

#[derive(Debug)]
pub struct ManagedBuffer {
    pub buffer: Buffer,
    pub mem_slot: usize,
    pub offset: u64,
    pub size: u64,
}

impl ManagedBuffer {
    pub const fn new(buffer: Buffer, mem_slot: usize, offset: u64, size: u64) -> Self {
        Self {
            buffer,
            mem_slot,
            offset,
            size,
        }
    }
}

#[derive(Debug)]
pub struct ManagedImage {
    pub image: Image,
    pub mem_slot: usize,
    pub offset: u64,
    pub size: u64,
}

impl ManagedImage {
    pub const fn new(image: Image, mem_slot: usize, offset: u64, size: u64) -> Self {
        Self {
            image,
            mem_slot,
            offset,
            size,
        }
    }
}
