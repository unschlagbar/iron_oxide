use ash::{
    Device,
    vk::{self, Rect2D},
};
use winit::dpi::PhysicalSize;

use crate::{
    graphics::{Buffer, VertexDescription, VkBase},
    ui::ui_pipeline::Pipeline,
};

pub trait Material {
    fn pipeline(&self) -> &Pipeline;
    fn buffer(&mut self) -> &mut Buffer;
    fn destroy(&mut self, device: &Device) {
        self.pipeline().destroy(device);
        self.buffer().destroy(device);
    }

    fn size_of(&self) -> u32;

    fn add(&mut self, to_add: *const (), descriptor: u32, clip: Option<Rect2D>);
    fn clear(&mut self);

    fn update(&mut self, base: &VkBase, cmd_pool: vk::CommandPool);
    fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer, clip: Rect2D) -> bool;
}

pub struct Basic<T: VertexDescription + Copy> {
    pub buffer: Buffer,
    pub pipeline: Pipeline,
    groups: Vec<DrawGroup<T>>,
}

impl<T: VertexDescription + Copy> Material for Basic<T> {
    fn buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    fn add(&mut self, to_add: *const (), desc: u32, clip: Option<Rect2D>) {
        let to_add = unsafe { *(to_add as *mut T) };
        if let Some(group) = self
            .groups
            .iter_mut()
            .find(|x| x.desc == desc && x.clip == clip)
        {
            group.data.push(to_add);
        } else {
            self.groups.push(DrawGroup {
                desc,
                clip,
                data: vec![to_add],
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

    fn update(&mut self, base: &VkBase, cmd_pool: vk::CommandPool) {
        let mut buf = Vec::new();

        for batch in &mut self.groups {
            batch.offset = buf.len() as u32;
            batch.size = batch.data.len() as u32;

            buf.extend_from_slice(&batch.data);
        }

        self.buffer.destroy(&base.device);

        if !buf.is_empty() {
            self.buffer = Buffer::device_local_slow(
                &base,
                cmd_pool,
                &buf,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
        }
    }
}

impl<T: VertexDescription + Copy> Basic<T> {
    pub fn new(
        base: &VkBase,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor_set_layout: vk::DescriptorSetLayout,
        shaders: (&[u8], &[u8]),
    ) -> Box<Self> {
        Box::new(Self {
            buffer: Buffer::null(),
            pipeline: Pipeline::create_ui::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layout,
                shaders,
            ),
            groups: Vec::new(),
        })
    }
}

struct DrawGroup<T> {
    pub desc: u32,
    pub clip: Option<Rect2D>,
    pub data: Vec<T>,
    pub size: u32,
    pub offset: u32,
}
