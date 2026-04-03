use core::slice;

use pyronyx::vk;

use super::VkBase;

pub struct SinlgeTimeCommands;

impl SinlgeTimeCommands {
    #[inline]
    pub fn begin(base: &VkBase, cmd_pool: vk::CommandPool) -> vk::CommandBuffer {
        let allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: cmd_pool,
            level: vk::CommandBufferLevel::Primary,
            command_buffer_count: 1,
            ..Default::default()
        };

        let mut command_buffer = vk::CommandBuffer::default();

        unsafe {
            base.device
                .allocate_command_buffers(&allocate_info, slice::from_mut(&mut command_buffer))
                .unwrap()
        };

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::OneTimeSubmit,
            ..Default::default()
        };

        command_buffer.begin(&begin_info).unwrap();
        command_buffer
    }

    #[inline]
    pub fn rebegin(cmd_buf: vk::CommandBuffer) {
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::OneTimeSubmit,
            ..Default::default()
        };

        cmd_buf.begin(&begin_info).unwrap()
    }

    #[inline]
    pub fn end(base: &VkBase, cmd_pool: vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();
        base.graphics_queue.wait_idle().unwrap();
        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }

    #[inline]
    pub fn end_debug(base: &VkBase, cmd_pool: vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();
        let start_time = std::time::Instant::now();
        base.graphics_queue.wait_idle().unwrap();
        println!("time: {:?}", start_time.elapsed());
        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }

    #[inline]
    pub fn submit(base: &VkBase, cmd_buf: vk::CommandBuffer) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();
    }

    #[inline]
    pub fn end_after_submit(base: &VkBase, cmd_pool: vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        base.graphics_queue.wait_idle().unwrap();
        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }

    #[inline]
    pub fn free(base: &VkBase, cmd_pool: vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }

    #[inline]
    pub fn reset(base: &VkBase, cmd_buf: vk::CommandBuffer) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();

        base.graphics_queue.wait_idle().unwrap();
        cmd_buf.reset(vk::CommandBufferResetFlags::empty()).unwrap();
    }

    pub fn end_no_wait(base: &VkBase, cmd_pool: vk::CommandPool, cmd_buf: vk::CommandBuffer) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();

        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }

    pub fn end_fence_wait(
        base: &VkBase,
        cmd_pool: vk::CommandPool,
        cmd_buf: vk::CommandBuffer,
        fence: vk::Fence,
    ) {
        cmd_buf.end().unwrap();

        let submits = vk::SubmitInfo {
            command_buffer_count: 1,
            command_buffers: &cmd_buf.handle(),
            ..Default::default()
        };

        base.graphics_queue
            .submit(&[submits], vk::Fence::null())
            .unwrap();

        base.device
            .wait_for_fences(&[fence], true, u64::MAX)
            .unwrap();
        base.device.reset_fences(&[fence]).unwrap();
        base.device
            .free_command_buffers(cmd_pool, &[cmd_buf.handle()]);
    }
}
