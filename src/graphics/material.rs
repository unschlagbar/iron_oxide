use ash::vk::{self, Buffer, Rect2D};
use winit::dpi::PhysicalSize;

use crate::{
    graphics::{Pipeline, VertexDescription, VkBase},
    ui::materials::MatType,
};

use std::fmt;

#[cfg(debug_assertions)]
use std::any::TypeId;

#[derive(Debug)]
pub struct Material {
    pub buffer: Buffer,
    pub pipeline: Pipeline,
    #[cfg(debug_assertions)]
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
        shaders: (&[u8], &[u8]),
    ) -> Self {
        debug_assert!(align_of::<T>() >= 4);
        Self {
            buffer: Buffer::null(),
            pipeline: Pipeline::create_ui::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            #[cfg(debug_assertions)]
            instance_type: TypeId::of::<T>(),
            stride: size_of::<T>(),
            desc_set: vk::DescriptorSet::null(),
        }
    }
}

pub struct DrawBatch {
    pub clip: Option<Rect2D>,
    pub data: Vec<u8>,
    pub mat_type: MatType,
    pub size: u32,
    pub offset: u32,
    pub z_index: i16,
    pub z_end: i16,
    pub done: bool,
}

impl fmt::Debug for DrawBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawBatch")
            .field("clip", &self.clip.is_some())
            .field("material", &self.mat_type)
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("Z_end", &self.z_end)
            .finish()
    }
}
