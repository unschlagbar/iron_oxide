use std::any::Any;

use ash::vk;
use winit::dpi::PhysicalSize;

use crate::{
    graphics::{Buffer, VertexDescription, VkBase},
    ui::{
        materials::{Basic, Material},
        ui_pipeline::Pipeline,
    },
};

pub struct SingleImage<T: VertexDescription + Copy + 'static> {
    pub basic: Basic<T>,
    pub desc_set: vk::DescriptorSet,
}

impl<T: VertexDescription + Copy> Material for SingleImage<T> {
    fn pipeline(&self) -> &Pipeline {
        self.basic.pipeline()
    }

    fn buffer(&mut self) -> &mut Buffer {
        self.basic.buffer()
    }

    fn staging_buffer(&mut self) -> &mut Buffer {
        self.basic.staging_buffer()
    }

    fn size_of(&self) -> u32 {
        self.basic.size_of()
    }

    fn add(&mut self, to_add: &dyn Any, descriptor: u32, clip: Option<vk::Rect2D>) {
        self.basic.add(to_add, descriptor, clip);
    }

    fn clear(&mut self) {
        self.basic.clear();
    }

    fn update(&mut self, base: &VkBase, cmd_buf: vk::CommandBuffer) {
        self.basic.update(base, cmd_buf);
    }

    fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer, clip: vk::Rect2D) -> bool {
        unsafe {
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline().layout,
                1,
                &[self.desc_set],
                &[],
            );
        }
        self.basic.draw(device, cmd, clip)
    }
}

impl<T: VertexDescription + Copy> SingleImage<T> {
    pub fn new(
        base: &VkBase,
        window_size: PhysicalSize<u32>,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        desc_set: vk::DescriptorSet,
        shaders: (&[u8], &[u8]),
    ) -> Self {
        Self {
            basic: Basic::new(
                base,
                window_size,
                render_pass,
                descriptor_set_layouts,
                shaders,
            ),
            desc_set,
        }
    }
}
