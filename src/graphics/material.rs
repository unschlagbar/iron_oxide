use ash::vk::{self, Buffer, Rect2D};
use winit::dpi::PhysicalSize;

use crate::graphics::{Pipeline, VertexDescription, VkBase};

use std::{any::TypeId, fmt};

#[derive(Debug)]
pub struct Material {
    pub buffer: Buffer,
    pub buffer_size: u64,
    pub pipeline: Pipeline,
    pub instance_type: TypeId,
    // In u32
    pub stride: usize,
    pub desc_set: vk::DescriptorSet,
}

impl Material {
    pub fn destroy(&mut self, device: &ash::Device) {
        self.pipeline.destroy(device);
    }
}

impl Material {
    pub fn new<T: VertexDescription>(
        base: &VkBase,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        desc_set: vk::DescriptorSet,
        alpha: bool,
        shaders: (&[u8], &[u8]),
    ) -> Self {
        assert!(align_of::<T>() >= 4);
        Self {
            buffer: Buffer::null(),
            buffer_size: 0,
            pipeline: Pipeline::create_ui::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
                alpha,
            ),
            instance_type: TypeId::of::<T>(),
            stride: size_of::<T>(),
            desc_set,
        }
    }
}

pub struct DrawBatch {
    pub clip: Option<Rect2D>,
    pub data: Vec<u8>,
    pub size: u32,
    pub offset: u32,
}

impl fmt::Debug for DrawBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawBatch")
            .field("clip", &self.clip)
            .field("data_len", &self.data.len())
            .field("size", &self.size)
            .field("offset", &self.offset)
            .finish()
    }
}
