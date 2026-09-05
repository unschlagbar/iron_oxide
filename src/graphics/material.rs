use pyronyx::vk::{self, Buffer, Rect2D};

use crate::{
    graphics::{Pipeline, VertexDescription, VkBase},
    ui::materials::MatType,
};

use std::fmt;

#[cfg(debug_assertions)]
use std::any::TypeId;

#[derive(Debug)]
pub struct Material {
    pub instance_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    pub pipeline: Pipeline,
    // In u32
    pub stride: usize,
    pub desc_set: vk::DescriptorSet,

    #[cfg(debug_assertions)]
    pub instance_type: TypeId,
}

impl Material {
    pub fn destroy(&mut self, device: &vk::Device) {
        self.pipeline.destroy(device);
    }
}

impl Material {
    pub fn new<T: VertexDescription>(
        base: &VkBase,
        window_size: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: (&[u8], &[u8]),
    ) -> Self {
        debug_assert!(align_of::<T>() >= 4);
        Self {
            instance_buffer: Buffer::null(),
            vertex_buffer: Buffer::null(),
            index_buffer: Buffer::null(),

            pipeline: Pipeline::create_ui::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            stride: size_of::<T>(),
            desc_set: vk::DescriptorSet::null(),

            #[cfg(debug_assertions)]
            instance_type: TypeId::of::<T>(),
        }
    }
    pub fn new_slang<T: VertexDescription>(
        base: &VkBase,
        window_size: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: &[u8],
    ) -> Self {
        debug_assert!(align_of::<T>() >= 4);
        Self {
            instance_buffer: Buffer::null(),
            vertex_buffer: Buffer::null(),
            index_buffer: Buffer::null(),

            pipeline: Pipeline::create_ui_slang::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            stride: size_of::<T>(),
            desc_set: vk::DescriptorSet::null(),

            #[cfg(debug_assertions)]
            instance_type: TypeId::of::<T>(),
        }
    }

    pub fn new_slang_list<T: VertexDescription>(
        base: &VkBase,
        window_size: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: &[u8],
    ) -> Self {
        debug_assert!(align_of::<T>() >= 4);
        Self {
            instance_buffer: Buffer::null(),
            vertex_buffer: Buffer::null(),
            index_buffer: Buffer::null(),

            pipeline: Pipeline::create_ui_slang_list::<T>(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            stride: size_of::<T>(),
            desc_set: vk::DescriptorSet::null(),

            #[cfg(debug_assertions)]
            instance_type: TypeId::of::<T>(),
        }
    }
}

pub struct DrawBatch {
    pub clip: Option<Rect2D>,
    /// Set on a batch that draws nothing itself and stands for a rectangle the
    /// application fills with its own pipeline. See `Resources::draw_with`.
    pub custom: Option<CustomDraw>,
    pub instance_data: Vec<u8>,
    pub vertex_data: Vec<u8>,
    pub index_data: Vec<u32>,
    pub mat_type: MatType,
    pub size: u32,
    pub offset: u32,
    pub z_index: i16,
    pub z_end: i16,
    pub done: bool,
    pub first_index: u32,
    pub index_count: u32,
}

/// A rectangle the ui has laid out but does not draw: what the application is
/// handed when the draw order reaches it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomDraw {
    /// Which drawing this is, from the `Canvas` that reserved the space.
    pub id: u16,
    /// The element's box in physical pixels.
    pub area: Rect2D,
    /// What the element is clipped to — a scroll panel it sits in, or the whole
    /// window. Painting outside this leaks over whatever scrolled it away.
    pub clip: Rect2D,
}

impl fmt::Debug for DrawBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawBatch")
            .field("clip", &self.clip.is_some())
            .field("material", &self.mat_type)
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("index_count", &self.index_count)
            .field("Z_end", &self.z_end)
            .finish()
    }
}
