use ash::{
    Device,
    vk::{self, Rect2D},
};

use crate::{
    graphics::{Buffer, VkBase},
    ui::pipeline::Pipeline,
};

pub trait Material {
    fn pipeline(&self) -> &Pipeline;
    fn buffer(&mut self) -> &mut Buffer;
    fn staging_buffer(&mut self) -> &mut Buffer;
    fn destroy(&mut self, device: &Device) {
        self.pipeline().destroy(device);
        self.buffer().destroy(device);
        self.staging_buffer().destroy(device);
    }

    fn size_of(&self) -> u32;

    fn add(&mut self, to_add: *const (), descriptor: u32, clip: Option<Rect2D>);
    fn clear(&mut self);

    fn update(&mut self, base: &VkBase, cmd_buf: vk::CommandBuffer);
    fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer, clip: Rect2D) -> bool;
}
