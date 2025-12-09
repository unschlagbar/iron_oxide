use super::{SinlgeTimeCommands, VkBase};
use ash::{
    Device,
    vk::{
        self, BufferCopy, BufferCreateInfo, BufferDeviceAddressInfo, BufferUsageFlags,
        CommandBuffer, CommandPool, DeviceMemory, DeviceOrHostAddressConstKHR,
        DeviceOrHostAddressKHR, Handle, MemoryAllocateFlags, MemoryAllocateFlagsInfo,
        MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, SharingMode,
    },
};
use std::ptr::{copy_nonoverlapping, null_mut};

#[derive(Debug, Clone, Copy)]
pub struct Buffer {
    pub inner: vk::Buffer,
    pub mem: DeviceMemory,
    pub size: u64,
}

impl Buffer {
    #[track_caller]
    pub fn create(
        base: &VkBase,
        size: u64,
        usage: BufferUsageFlags,
        properties: MemoryPropertyFlags,
    ) -> Self {
        assert_ne!(size, 0, "Size must be larger than 0");
        let buffer_info = BufferCreateInfo {
            size,
            usage,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { base.device.create_buffer(&buffer_info, None).unwrap() };
        let mem_requirements = unsafe { base.device.get_buffer_memory_requirements(buffer) };

        let alloc_flags_info = MemoryAllocateFlagsInfo {
            flags: if usage.contains(BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
                MemoryAllocateFlags::DEVICE_ADDRESS
            } else {
                MemoryAllocateFlags::empty()
            },
            ..Default::default()
        };

        let alloc_info = MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: find_memory_type(
                base,
                mem_requirements.memory_type_bits,
                properties,
            ),
            p_next: &alloc_flags_info as *const _ as *const _,
            ..Default::default()
        };

        let mem = unsafe { base.device.allocate_memory(&alloc_info, None).unwrap() };

        unsafe { base.device.bind_buffer_memory(buffer, mem, 0).unwrap() };
        Self {
            inner: buffer,
            mem,
            size,
        }
    }

    pub fn create_stagging(base: &VkBase, size: u64) -> Self {
        Self::create(
            base,
            size,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
    }

    pub fn create_uniform<T, const MFIF: usize>(
        base: &VkBase,
    ) -> ([Buffer; MFIF], [*mut T; MFIF]) {
        let buffer_size = std::mem::size_of::<T>() as u64;

        let mut uniform_buffers = [Buffer::null(); MFIF];
        let mut mapped = [null_mut(); MFIF];

        for i in 0..MFIF {
            uniform_buffers[i] = Buffer::create(
                base,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            mapped[i] = uniform_buffers[i].map_memory(&base.device, buffer_size, 0);
        }

        (uniform_buffers, mapped)
    }

    pub fn null() -> Self {
        Self {
            inner: vk::Buffer::null(),
            mem: DeviceMemory::null(),
            size: 0,
        }
    }

    pub fn update_data<T>(&mut self, base: &VkBase, data: &[T], offset: u64) {
        let new_data_size = size_of::<T>() as u64 * data.len() as u64;
        assert!(new_data_size + offset > self.size);

        let mapped_memory = self.map_memory(&base.device, self.size, offset);
        unsafe {
            copy_nonoverlapping(data.as_ptr(), mapped_memory, data.len());
            self.unmap_memory(&base.device);
        };
    }

    pub fn update<T>(
        &mut self,
        base: &VkBase,
        cmd_pool: CommandPool,
        data: &[T],
        usage: BufferUsageFlags,
    ) {
        let buffer_size = size_of::<T>() as u64 * data.len() as u64;
        let mut staging_buffer = Self::create_stagging(base, buffer_size);

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            copy_nonoverlapping(data.as_ptr(), mapped_memory, data.len());
            staging_buffer.unmap_memory(&base.device);
        };

        if self.size < buffer_size {
            unsafe { base.device.queue_wait_idle(base.queue).unwrap_unchecked() };
            self.destroy(&base.device);
            *self = Self::create(
                base,
                buffer_size,
                usage | BufferUsageFlags::TRANSFER_DST,
                MemoryPropertyFlags::DEVICE_LOCAL,
            );
        }

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);
        staging_buffer.copy(base, self, buffer_size, 0, cmd_buf);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);

        staging_buffer.destroy(&base.device);
    }

    pub fn update_on_cmd_buf<T>(
        &mut self,
        base: &VkBase,
        staging_buffer: &Self,
        data: &[T],
        cmd_buf: CommandBuffer,
    ) {
        let buffer_size = size_of::<T>() as u64 * data.len() as u64;

        assert!(buffer_size <= self.size && buffer_size <= staging_buffer.size);

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            copy_nonoverlapping(data.as_ptr(), mapped_memory, data.len());
            staging_buffer.unmap_memory(&base.device);
        };

        staging_buffer.copy(base, self, buffer_size, 0, cmd_buf);
    }

    #[track_caller]
    ///Very slow!!!
    pub fn upload<T>(
        base: &VkBase,
        cmd_pool: CommandPool,
        data: &[T],
        usage: BufferUsageFlags,
    ) -> Self {
        let buffer_size = data.len() as u64 * size_of::<T>() as u64;
        let mut staging_buffer = Self::create_stagging(base, buffer_size);

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            copy_nonoverlapping(data.as_ptr(), mapped_memory, data.len());
        };
        staging_buffer.unmap_memory(&base.device);

        let device_local_buffer = Self::create(
            base,
            buffer_size,
            usage | BufferUsageFlags::TRANSFER_DST,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);
        staging_buffer.copy(base, &device_local_buffer, buffer_size, 0, cmd_buf);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);

        staging_buffer.destroy(&base.device);

        device_local_buffer
    }

    pub fn copy(
        &self,
        base: &VkBase,
        dst_buffer: &Self,
        size: u64,
        src_offset: u64,
        cmd_buf: CommandBuffer,
    ) {
        let copy_region = BufferCopy {
            src_offset,
            dst_offset: 0,
            size,
        };

        unsafe {
            base.device
                .cmd_copy_buffer(cmd_buf, self.inner, dst_buffer.inner, &[copy_region])
        };
    }

    #[inline]
    pub fn map_memory<T>(&self, device: &Device, buffer_size: u64, offset: u64) -> *mut T {
        unsafe {
            device
                .map_memory(self.mem, offset, buffer_size, MemoryMapFlags::empty())
                .unwrap() as _
        }
    }

    #[inline]
    pub fn unmap_memory(&self, device: &Device) {
        unsafe { device.unmap_memory(self.mem) };
    }

    #[inline]
    pub fn get_device_addr_u64(&self, device: &Device) -> u64 {
        let device_address_info = BufferDeviceAddressInfo {
            buffer: self.inner,
            ..Default::default()
        };

        unsafe { device.get_buffer_device_address(&device_address_info) }
    }

    #[inline]
    pub fn get_device_addr(&self, device: &Device) -> DeviceOrHostAddressKHR {
        DeviceOrHostAddressKHR {
            device_address: self.get_device_addr_u64(device),
        }
    }

    #[inline]
    pub fn get_device_addr_const(&self, device: &Device) -> DeviceOrHostAddressConstKHR {
        DeviceOrHostAddressConstKHR {
            device_address: self.get_device_addr_u64(device),
        }
    }

    #[inline]
    pub fn destroy(&mut self, device: &Device) {
        if !self.inner.is_null() {
            unsafe {
                device.destroy_buffer(self.inner, None);
                device.free_memory(self.mem, None);
            }
            self.inner = vk::Buffer::null();
        }
    }
}

pub fn find_memory_type(base: &VkBase, type_filter: u32, properties: MemoryPropertyFlags) -> u32 {
    let mem_properties = unsafe {
        base.instance
            .get_physical_device_memory_properties(base.physical_device)
    };

    //println!("mem: {:?}", mem_properties);

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i) != 0)
            && (mem_properties.memory_types[i as usize].property_flags & properties) == properties
        {
            return i;
        }
    }

    panic!("Can not find memory type!");
}
