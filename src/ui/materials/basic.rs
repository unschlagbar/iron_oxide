use ash::vk::{self, Rect2D};
use winit::dpi::PhysicalSize;

use crate::{
    graphics::{Buffer, VertexDescription, VkBase},
    ui::{materials::Material, pipeline::Pipeline},
};

pub struct Basic<T: VertexDescription> {
    pub buffer: Buffer,
    pub staging_buffer: Buffer,
    pub pipeline: Pipeline,
    groups: Vec<BasicDrawGroup<T>>,
}

impl<T: VertexDescription> Material for Basic<T> {
    fn buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn staging_buffer(&mut self) -> &mut Buffer {
        &mut self.staging_buffer
    }

    fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    fn add(&mut self, to_add: *const (), _: u32, clip: Option<Rect2D>) {
        let v = unsafe { &*to_add.cast::<T>() };
        if let Some(group) = self.groups.iter_mut().find(|x| x.clip == clip) {
            group.data.push(*v);
        } else {
            self.groups.push(BasicDrawGroup {
                //desc,
                clip,
                data: vec![*v],
                size: 0,
                offset: 0,
            });
        }
    }

    fn clear(&mut self) {
        self.groups.clear();
    }

    fn size_of(&self) -> u32 {
        size_of::<T>() as _
    }

    fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer, clip: Rect2D) -> bool {
        if self.groups.is_empty() {
            return false;
        }

        let mut last_had_clip = false;
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline.this);
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.buffer.inner], &[0]);

            for batch in &self.groups {
                if let Some(clip) = batch.clip {
                    device.cmd_set_scissor(cmd, 0, &[clip]);
                    last_had_clip = true;
                } else if last_had_clip {
                    device.cmd_set_scissor(cmd, 0, &[clip]);
                    last_had_clip = false;
                }
                device.cmd_draw(cmd, 4, batch.size, 0, batch.offset);
            }
        }
        last_had_clip
    }

    fn update(&mut self, base: &VkBase, cmd_buf: vk::CommandBuffer) {
        let mut buf = Vec::new();

        for batch in &mut self.groups {
            batch.offset = buf.len() as u32;
            batch.size = batch.data.len() as u32;

            buf.extend_from_slice(&batch.data);
        }

        if buf.is_empty() {
            return;
        }

        if buf.len() * size_of::<T>() > self.staging_buffer.size as usize {
            unsafe { base.device.queue_wait_idle(base.queue).unwrap() };

            self.staging_buffer.destroy(&base.device);
            self.buffer.destroy(&base.device);

            let size = (size_of::<T>() * (buf.len() + 20)) as u64;

            self.buffer = Buffer::create(
                base,
                size,
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );
            self.staging_buffer = Buffer::create_stagging(base, size);
        }

        self.buffer
            .update_on_cmd_buf(base, &self.staging_buffer, &buf, cmd_buf);
    }
}

impl<T: VertexDescription> Basic<T> {
    pub fn new(
        base: &VkBase,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: (&[u8], &[u8]),
    ) -> Self {
        Self {
            buffer: Buffer::null(),
            staging_buffer: Buffer::null(),
            pipeline: Pipeline::create_ui::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            groups: Vec::new(),
        }
    }
}

struct BasicDrawGroup<T> {
    //pub desc: u32,
    pub clip: Option<Rect2D>,
    pub data: Vec<T>,
    pub size: u32,
    pub offset: u32,
}
