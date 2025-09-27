use super::{SinlgeTimeCommands, VkBase};
use ash::vk::{self, Handle, MemoryAllocateFlags};
use std::{ffi::c_void, ptr};

#[derive(Debug, Clone, Copy)]
pub struct Buffer {
    pub inner: vk::Buffer,
    pub mem: vk::DeviceMemory,
    pub size: u64,
}

impl Buffer {
    pub fn create(
        base: &VkBase,
        size: u64,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        assert_ne!(size, 0, "Buffer must have a size larger than 0");
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            base.device
                .create_buffer(&buffer_info, None)
                .unwrap_unchecked()
        };
        let mem_requirements = unsafe { base.device.get_buffer_memory_requirements(buffer) };

        let alloc_flags_info = vk::MemoryAllocateFlagsInfo {
            flags: if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
                vk::MemoryAllocateFlags::DEVICE_ADDRESS
            } else {
                MemoryAllocateFlags::empty()
            }, // Aktiviert die Nutzung von Geräteadressen
            ..Default::default()
        };

        let alloc_info = vk::MemoryAllocateInfo {
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

        unsafe {
            base.device
                .bind_buffer_memory(buffer, mem, 0)
                .unwrap_unchecked()
        };
        Self {
            inner: buffer,
            mem,
            size,
        }
    }

    pub fn null() -> Self {
        Self {
            inner: vk::Buffer::null(),
            mem: vk::DeviceMemory::null(),
            size: 0,
        }
    }

    pub fn device_local_raw(
        base: &VkBase,
        command_pool: vk::CommandPool,
        stride: u64,
        len: u64,
        data: *const u8,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let buffer_size = stride * len;
        let staging_buffer = Self::create(
            base,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            std::ptr::copy_nonoverlapping(data, mapped_memory as _, buffer_size as usize);
            staging_buffer.unmap_memory(&base.device);
        };

        let device_local_buffer = Self::create(
            base,
            buffer_size,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let cmd_buf = SinlgeTimeCommands::begin(&base, command_pool);
        staging_buffer.copy(base, &device_local_buffer, buffer_size, 0, cmd_buf);
        SinlgeTimeCommands::end(base, command_pool, cmd_buf);

        staging_buffer.destroy(&base.device);

        device_local_buffer
    }

    pub fn update_data<T>(&mut self, base: &VkBase, data: &[T], offset: u64) {
        let new_data_size = size_of::<T>() as u64 * data.len() as u64;
        assert!(new_data_size + offset > self.size);

        let mapped_memory = self.map_memory(&base.device, self.size, offset);
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), mapped_memory as _, data.len());
            self.unmap_memory(&base.device);
        };
    }

    pub fn update<T>(
        &mut self,
        base: &VkBase,
        command_pool: vk::CommandPool,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) {
        let buffer_size = size_of::<T>() as u64 * data.len() as u64;
        let staging_buffer = Self::create(
            base,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), mapped_memory as _, data.len());
            staging_buffer.unmap_memory(&base.device);
        };

        if self.size < buffer_size {
            unsafe { base.device.queue_wait_idle(base.queue).unwrap_unchecked() };
            self.destroy(&base.device);
            *self = Self::create(
                base,
                buffer_size,
                usage | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );
        }

        let cmd_buf = SinlgeTimeCommands::begin(&base, command_pool);
        staging_buffer.copy(base, &self, buffer_size, 0, cmd_buf);
        SinlgeTimeCommands::end(base, command_pool, cmd_buf);

        staging_buffer.destroy(&base.device);
    }

    pub fn device_local_slow<T>(
        base: &VkBase,
        command_pool: vk::CommandPool,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let buffer_size = data.len() as u64 * size_of::<T>() as u64;
        let staging_buffer = Self::create(
            base,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let mapped_memory = staging_buffer.map_memory(&base.device, buffer_size, 0);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_memory as _, data.len());
        };
        staging_buffer.unmap_memory(&base.device);

        let device_local_buffer = Self::create(
            base,
            buffer_size,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let cmd_buf = SinlgeTimeCommands::begin(&base, command_pool);
        staging_buffer.copy(base, &device_local_buffer, buffer_size, 0, cmd_buf);
        SinlgeTimeCommands::end(base, command_pool, cmd_buf);

        staging_buffer.destroy(&base.device);

        device_local_buffer
    }

    pub fn copy(
        &self,
        base: &VkBase,
        dst_buffer: &Self,
        size: u64,
        src_offset: u64,
        cmd_buf: vk::CommandBuffer,
    ) {
        let copy_region = vk::BufferCopy {
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
    pub fn map_memory(&self, device: &ash::Device, buffer_size: u64, offset: u64) -> *const c_void {
        unsafe {
            device
                .map_memory(self.mem, offset, buffer_size, vk::MemoryMapFlags::empty())
                .unwrap()
        }
    }

    #[inline]
    pub fn unmap_memory(&self, device: &ash::Device) {
        unsafe { device.unmap_memory(self.mem) };
    }

    #[inline]
    pub fn get_device_addr(&self, device: &ash::Device) -> vk::DeviceOrHostAddressKHR {
        let device_address_info = vk::BufferDeviceAddressInfo {
            buffer: self.inner,
            ..Default::default()
        };

        let device_address = unsafe { device.get_buffer_device_address(&device_address_info) };

        vk::DeviceOrHostAddressKHR { device_address }
    }

    #[inline]
    pub fn get_device_addr_u64(&self, device: &ash::Device) -> u64 {
        let device_address_info = vk::BufferDeviceAddressInfo {
            buffer: self.inner,
            ..Default::default()
        };

        unsafe { device.get_buffer_device_address(&device_address_info) }
    }

    #[inline]
    pub fn get_device_addr_const(&self, device: &ash::Device) -> vk::DeviceOrHostAddressConstKHR {
        let device_address_info = vk::BufferDeviceAddressInfo {
            buffer: self.inner,
            ..Default::default()
        };

        let device_address = unsafe { device.get_buffer_device_address(&device_address_info) };

        vk::DeviceOrHostAddressConstKHR { device_address }
    }

    #[inline]
    pub fn destroy(&self, device: &ash::Device) {
        if !self.inner.is_null() {
            unsafe {
                device.destroy_buffer(self.inner, None);
                device.free_memory(self.mem, None);
            }
        }
    }
}

pub fn find_memory_type(
    base: &VkBase,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    let mem_properties = unsafe {
        base.instance
            .get_physical_device_memory_properties(base.physical_device)
    };

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i) != 0)
            && (mem_properties.memory_types[i as usize].property_flags & properties) == properties
        {
            return i;
        }
    }

    panic!("Can not find memory type!");
}
