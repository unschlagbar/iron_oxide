use ash::vk;

use super::VkBase;

pub struct SinlgeTimeCommands;

impl SinlgeTimeCommands {
    #[inline]
    pub fn begin(base: &VkBase, command_pool: &vk::CommandPool) -> vk::CommandBuffer {
        let allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: *command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        let command_buffer = unsafe { base.device.allocate_command_buffers(&allocate_info).unwrap()[0] };

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe { base.device.begin_command_buffer(command_buffer, &begin_info).unwrap_unchecked() };
        command_buffer
    }

    #[inline]
    pub fn rebegin(base: &VkBase, cmd_buf: &vk::CommandBuffer) {

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe { base.device.begin_command_buffer(*cmd_buf, &begin_info).unwrap_unchecked() };
    }

    #[inline]
    pub fn end(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        unsafe { base.device.end_command_buffer(cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], vk::Fence::null()).unwrap_unchecked();
            base.device.queue_wait_idle(base.queue).unwrap_unchecked();
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }

    #[inline]
    pub fn end_debug(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        unsafe { base.device.end_command_buffer(cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], vk::Fence::null()).unwrap_unchecked();
            let start_time = std::time::Instant::now();
            base.device.queue_wait_idle(base.queue).unwrap_unchecked();
            println!("time: {:?}", start_time.elapsed());
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }

    #[inline]
    pub fn submit(base: &VkBase, cmd_buf: vk::CommandBuffer) {
        unsafe { base.device.end_command_buffer(cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], vk::Fence::null()).unwrap_unchecked();
        }
    }

    #[inline]
    pub fn end_after_submit(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        unsafe { 
            base.device.queue_wait_idle(base.queue).unwrap_unchecked();
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }

    #[inline]
    pub fn free(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        unsafe { 
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }

    #[inline]
    pub fn reset(base: &VkBase, cmd_buf: &vk::CommandBuffer) {
        unsafe { base.device.end_command_buffer(*cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], vk::Fence::null()).unwrap_unchecked();
            base.device.queue_wait_idle(base.queue).unwrap_unchecked();
            base.device.reset_command_buffer(*cmd_buf, vk::CommandBufferResetFlags::empty()).unwrap_unchecked();
        }
    }

    pub fn end_no_wait(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        unsafe { base.device.end_command_buffer(cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], vk::Fence::null()).unwrap_unchecked();
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }

    pub fn end_fence_wait(base: &VkBase, command_pool: &vk::CommandPool, cmd_buf: vk::CommandBuffer, fence: vk::Fence) {
        unsafe { base.device.end_command_buffer(cmd_buf).unwrap_unchecked() }

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &cmd_buf,
            ..Default::default()
        };

        unsafe { 
            base.device.queue_submit(base.queue, &[submits], fence).unwrap_unchecked();
            base.device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
            base.device.reset_fences(&[fence]).unwrap();
            base.device.free_command_buffers(*command_pool, &[cmd_buf]);
        }
    }
}