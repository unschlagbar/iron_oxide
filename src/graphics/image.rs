use std::{fmt::Debug, fs::File, io::BufWriter};

use ash::vk::{
    AccessFlags, Buffer, BufferImageCopy, BufferUsageFlags, CommandBuffer, CommandPool,
    DependencyFlags, Extent3D, Format, Handle, Image, ImageAspectFlags, ImageCreateInfo,
    ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageView,
    ImageViewCreateInfo, ImageViewType, Offset3D, PipelineStageFlags,
};
use png::Encoder;

use crate::graphics::{Ressources, SinlgeTimeCommands};

use super::VkBase;

#[derive(Debug, Default, Clone)]
pub struct VulkanImage {
    pub image: Image,
    pub view: ImageView,
    pub format: Format,
    pub layout: ImageLayout,
    pub extent: Extent3D,
}

impl VulkanImage {
    pub fn create(base: &VkBase, create_info: &ImageCreateInfo) -> Self {
        let image = unsafe { base.device.create_image(create_info, None).unwrap() };

        let format = create_info.format;
        let layout = create_info.initial_layout;
        let extent = create_info.extent;
        Self {
            image,
            view: ImageView::null(),
            format,
            layout,
            extent,
        }
    }

    pub fn aspect_flags(&self) -> ImageAspectFlags {
        match self.format {
            Format::D16_UNORM
            | Format::D16_UNORM_S8_UINT
            | Format::D24_UNORM_S8_UINT
            | Format::D32_SFLOAT
            | Format::D32_SFLOAT_S8_UINT => ImageAspectFlags::DEPTH,
            _ => ImageAspectFlags::COLOR,
        }
    }

    pub fn trasition_layout(
        &mut self,
        base: &VkBase,
        cmd_buf: CommandBuffer,
        new_layout: ImageLayout,
    ) {
        let mut barrier = ImageMemoryBarrier {
            old_layout: self.layout,
            new_layout,
            image: self.image,
            subresource_range: ImageSubresourceRange {
                aspect_mask: {
                    if self.format == Format::D32_SFLOAT || self.format == Format::D16_UNORM {
                        ImageAspectFlags::DEPTH
                    } else if self.format == Format::D24_UNORM_S8_UINT {
                        ImageAspectFlags::DEPTH | ImageAspectFlags::STENCIL
                    } else {
                        ImageAspectFlags::COLOR
                    }
                },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        let source_stage;
        let destination_stage;

        match (new_layout, self.layout) {
            (ImageLayout::TRANSFER_DST_OPTIMAL, ImageLayout::UNDEFINED) => {
                barrier.src_access_mask = AccessFlags::NONE;
                barrier.dst_access_mask = AccessFlags::TRANSFER_WRITE;

                source_stage = PipelineStageFlags::TOP_OF_PIPE;
                destination_stage = PipelineStageFlags::TRANSFER;
            }
            (
                ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ImageLayout::TRANSFER_DST_OPTIMAL | ImageLayout::TRANSFER_SRC_OPTIMAL,
            ) => {
                barrier.src_access_mask = AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = AccessFlags::SHADER_READ;

                source_stage = PipelineStageFlags::TRANSFER;
                destination_stage = PipelineStageFlags::FRAGMENT_SHADER;
            }
            (ImageLayout::GENERAL, ImageLayout::UNDEFINED) => {
                barrier.src_access_mask = AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = AccessFlags::SHADER_READ;

                source_stage = PipelineStageFlags::TRANSFER;
                destination_stage = PipelineStageFlags::FRAGMENT_SHADER;
            }
            (ImageLayout::TRANSFER_SRC_OPTIMAL, ImageLayout::UNDEFINED) => {
                barrier.src_access_mask = AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = AccessFlags::SHADER_READ;

                source_stage = PipelineStageFlags::TRANSFER;
                destination_stage = PipelineStageFlags::FRAGMENT_SHADER;
            }
            (ImageLayout::TRANSFER_SRC_OPTIMAL, ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                barrier.src_access_mask = AccessFlags::SHADER_READ;
                barrier.dst_access_mask = AccessFlags::TRANSFER_READ;

                source_stage = PipelineStageFlags::FRAGMENT_SHADER;
                destination_stage = PipelineStageFlags::TRANSFER;
            }
            (ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, ImageLayout::UNDEFINED) => {
                barrier.src_access_mask = AccessFlags::NONE;
                barrier.dst_access_mask = AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;

                source_stage = PipelineStageFlags::TOP_OF_PIPE;
                destination_stage = PipelineStageFlags::EARLY_FRAGMENT_TESTS;
            }
            _ => panic!(
                "From layout: {:?} to layout: {:?} is not implemented!",
                self.layout, new_layout
            ),
        }
        self.layout = new_layout;
        unsafe {
            base.device.cmd_pipeline_barrier(
                cmd_buf,
                source_stage,
                destination_stage,
                DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        }
    }

    pub fn copy_from_buffer(&self, base: &VkBase, cmd_buf: CommandBuffer, buffer: Buffer) {
        let region = BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers {
                aspect_mask: self.aspect_flags(),
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: Offset3D::default(),
            image_extent: self.extent,
        };

        unsafe {
            base.device.cmd_copy_buffer_to_image(
                cmd_buf,
                buffer,
                self.image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            )
        };
    }

    pub fn create_view(&mut self, base: &VkBase) {
        let create_info = ImageViewCreateInfo {
            image: self.image,
            view_type: ImageViewType::TYPE_2D,
            format: self.format,
            subresource_range: ImageSubresourceRange {
                aspect_mask: self.aspect_flags(),
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        self.view = unsafe { base.device.create_image_view(&create_info, None).unwrap() }
    }

    pub fn save_to_png(
        &self,
        base: &VkBase,
        ressources: &mut Ressources,
        cmd_pool: CommandPool,
        extent: Extent3D,
        path: &str,
    ) {
        let size = extent.width as usize * extent.height as usize * 4;

        let (staging_buffer, offset) = ressources.mem_manager.create_buffer(
            base,
            0,
            size as u64,
            BufferUsageFlags::TRANSFER_DST,
        );
        let mut img = self.clone();

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);

        img.trasition_layout(base, cmd_buf, ImageLayout::TRANSFER_SRC_OPTIMAL);

        let region = BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: Offset3D::default(),
            image_extent: extent,
        };
        unsafe {
            base.device.cmd_copy_image_to_buffer(
                cmd_buf,
                self.image,
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                staging_buffer,
                &[region],
            );
        }

        img.trasition_layout(base, cmd_buf, self.layout);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);

        let mut buf: Vec<u8> = Vec::with_capacity(size);

        unsafe {
            let src = ressources.mem_manager.memory_pool[0].get_ptr(offset as usize);
            buf.as_mut_ptr()
                .copy_from_nonoverlapping(src as *const u8, size);
            buf.set_len(size);
        };

        // 6. PNG schreiben
        let file = File::create(path).expect("Failed to create PNG file");
        let mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(&mut w, extent.width, extent.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fastest);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&buf).unwrap();

        ressources.mem_manager.pop_buffer(base);
    }

    pub fn destroy_view(&mut self, device: &ash::Device) {
        if !self.view.is_null() {
            unsafe { device.destroy_image_view(self.view, None) };
            self.view = ImageView::null()
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        if !self.view.is_null() {
            unsafe {
                device.destroy_image_view(self.view, None);
                device.destroy_image(self.image, None);
            };
            self.view = ImageView::null();
            self.image = Image::null();
        }
    }
}
