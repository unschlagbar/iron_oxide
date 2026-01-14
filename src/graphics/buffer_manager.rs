use std::u32;

use ash::vk::{
    Buffer, BufferCreateInfo, BufferUsageFlags, DeviceMemory, MemoryAllocateInfo,
    MemoryPropertyFlags, SharingMode,
};

use crate::graphics::VkBase;

#[derive(Debug)]
pub struct BufferManager {
    pub is_uma: bool,
    pub host_visible: u32,
    pub device_local: u32,

    pub memory_pool: Vec<Memory>,
    pub buffers: Vec<Buffer>,
}

impl BufferManager {
    pub fn new(base: &VkBase) -> Self {
        let mem_properties = unsafe {
            base.instance
                .get_physical_device_memory_properties(base.physical_device)
        };

        let mut host_visible = u32::MAX;
        let mut device_local = u32::MAX;

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
        }

        if host_visible == u32::MAX || device_local == u32::MAX {
            panic!("Missing memory Types");
        }

        let is_uma = host_visible == device_local;

        Self {
            is_uma,
            host_visible,
            device_local,
            memory_pool: Vec::new(),
            buffers: Vec::new(),
        }
    }

    pub fn allocate_memory(&mut self, base: &VkBase, memory_type_index: u32, allocation_size: u64) {
        let alloc_info = MemoryAllocateInfo {
            allocation_size,
            memory_type_index,
            ..Default::default()
        };

        let mem = unsafe { base.device.allocate_memory(&alloc_info, None).unwrap() };
        self.memory_pool.push(Memory {
            mem_type_idx: memory_type_index,
            size: allocation_size,
            used: 0,
            memory: mem,
        });
    }

    pub fn create_buffer(
        &mut self,
        base: &VkBase,
        size: u64,
        usage: BufferUsageFlags,
    ) -> (Buffer, u64) {
        assert_ne!(size, 0, "Size must be larger than 0");

        let buffer_info = BufferCreateInfo {
            size,
            usage,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { base.device.create_buffer(&buffer_info, None).unwrap() };
        let mem_requirements = unsafe { base.device.get_buffer_memory_requirements(buffer) };

        let alignment = mem_requirements.alignment;
        let buffer_size = (mem_requirements.size + alignment - 1) & !(alignment - 1);

        let mem = &mut self.memory_pool[0];
        assert!(mem.correct_mem_type(mem_requirements.memory_type_bits));
        mem.bind_buffer(base, buffer, buffer_size);

        self.buffers.push(buffer);
        (buffer, buffer_size)
    }

    pub fn destroy_buffers(&mut self, base: &VkBase) {
        self.memory_pool[0].used = 0;
        unsafe {
            self.buffers
                .drain(..)
                .for_each(|buffer| base.device.destroy_buffer(buffer, None));
        }
    }

    pub fn destroy(&mut self, base: &VkBase) {
        self.destroy_buffers(base);
        self.memory_pool
            .drain(..)
            .for_each(|memory| memory.destroy(base));
    }
}

#[derive(Debug)]
pub struct Memory {
    pub mem_type_idx: u32,
    pub size: u64,
    pub used: u64,
    pub memory: DeviceMemory,
}

impl Memory {
    pub fn destroy(self, base: &VkBase) {
        unsafe {
            base.device.free_memory(self.memory, None);
        }
    }

    pub fn bind_buffer(&mut self, base: &VkBase, buffer: Buffer, buffer_size: u64) {
        unsafe {
            base.device
                .bind_buffer_memory(buffer, self.memory, self.used)
                .unwrap()
        };
        self.used += buffer_size;
    }

    pub fn correct_mem_type(&self, memory_type_bits: u32) -> bool {
        memory_type_bits & (1 << self.mem_type_idx) != 0
    }
}
