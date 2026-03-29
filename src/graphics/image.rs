use png::Encoder;
use pyronyx::vk::{
    AccessFlags, Buffer, BufferImageCopy, BufferUsageFlags, CommandBuffer, CommandPool,
    DependencyFlags, Device, Extent3D, Format, Image, ImageAspectFlags, ImageCreateInfo,
    ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageView,
    ImageViewCreateInfo, ImageViewType, Offset3D, PipelineStageFlags,
};
use std::{fmt::Debug, fs::File, io::BufWriter};

use crate::graphics::{Resources, SinlgeTimeCommands};

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
        let image = base.device.create_image(create_info, None).unwrap();

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
            Format::D16Unorm
            | Format::D16UnormS8Uint
            | Format::D24UnormS8Uint
            | Format::D32Sfloat
            | Format::D32SfloatS8Uint => ImageAspectFlags::Depth,
            _ => ImageAspectFlags::Color,
        }
    }

    pub fn trasition_layout(&mut self, cmd_buf: CommandBuffer, new_layout: ImageLayout) {
        let mut barrier = ImageMemoryBarrier {
            old_layout: self.layout,
            new_layout,
            image: self.image,
            subresource_range: ImageSubresourceRange {
                aspect_mask: {
                    if self.format == Format::D32Sfloat || self.format == Format::D16Unorm {
                        ImageAspectFlags::Depth
                    } else if self.format == Format::D24UnormS8Uint {
                        ImageAspectFlags::Depth | ImageAspectFlags::Stencil
                    } else {
                        ImageAspectFlags::Color
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
            (ImageLayout::TransferDstOptimal, ImageLayout::Undefined) => {
                barrier.src_access_mask = AccessFlags::None;
                barrier.dst_access_mask = AccessFlags::TransferWrite;

                source_stage = PipelineStageFlags::TopOfPipe;
                destination_stage = PipelineStageFlags::Transfer;
            }
            (
                ImageLayout::ShaderReadOnlyOptimal,
                ImageLayout::TransferDstOptimal | ImageLayout::TransferSrcOptimal,
            ) => {
                barrier.src_access_mask = AccessFlags::TransferWrite;
                barrier.dst_access_mask = AccessFlags::ShaderRead;

                source_stage = PipelineStageFlags::Transfer;
                destination_stage = PipelineStageFlags::FragmentShader;
            }
            (ImageLayout::General, ImageLayout::Undefined) => {
                barrier.src_access_mask = AccessFlags::TransferWrite;
                barrier.dst_access_mask = AccessFlags::ShaderRead;

                source_stage = PipelineStageFlags::Transfer;
                destination_stage = PipelineStageFlags::FragmentShader;
            }
            (ImageLayout::TransferSrcOptimal, ImageLayout::Undefined) => {
                barrier.src_access_mask = AccessFlags::TransferWrite;
                barrier.dst_access_mask = AccessFlags::ShaderRead;

                source_stage = PipelineStageFlags::Transfer;
                destination_stage = PipelineStageFlags::FragmentShader;
            }
            (ImageLayout::TransferSrcOptimal, ImageLayout::ShaderReadOnlyOptimal) => {
                barrier.src_access_mask = AccessFlags::ShaderRead;
                barrier.dst_access_mask = AccessFlags::TransferRead;

                source_stage = PipelineStageFlags::FragmentShader;
                destination_stage = PipelineStageFlags::Transfer;
            }
            (ImageLayout::DepthAttachmentOptimal, ImageLayout::Undefined) => {
                barrier.src_access_mask = AccessFlags::None;
                barrier.dst_access_mask = AccessFlags::DepthStencilAttachmentRead
                    | AccessFlags::DepthStencilAttachmentWrite;

                source_stage = PipelineStageFlags::TopOfPipe;
                destination_stage = PipelineStageFlags::EarlyFragmentTests;
            }
            _ => panic!(
                "From layout: {:?} to layout: {:?} is not implemented!",
                self.layout, new_layout
            ),
        }
        self.layout = new_layout;
        cmd_buf.pipeline_barrier(
            source_stage,
            destination_stage,
            DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        )
    }

    pub fn copy_from_buffer(&self, cmd_buf: CommandBuffer, buffer: Buffer) {
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

        cmd_buf.copy_buffer_to_image(
            buffer,
            self.image,
            ImageLayout::TransferDstOptimal,
            &[region],
        )
    }

    pub fn create_view(&mut self, base: &VkBase) {
        let create_info = ImageViewCreateInfo {
            image: self.image,
            view_type: ImageViewType::Type2d,
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
        self.view = base.device.create_image_view(&create_info, None).unwrap()
    }

    pub fn save_to_png(
        &self,
        base: &VkBase,
        resources: &mut Resources,
        cmd_pool: CommandPool,
        extent: Extent3D,
        path: &str,
    ) {
        let size = extent.width as usize * extent.height as usize * 4;

        let (staging_buffer, offset) = resources.mem_manager.create_buffer(
            base,
            0,
            size as u64,
            BufferUsageFlags::TransferDst,
        );
        let mut img = self.clone();

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);

        img.trasition_layout(cmd_buf, ImageLayout::TransferSrcOptimal);

        let region = BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::Color,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: Offset3D::default(),
            image_extent: extent,
        };
        cmd_buf.copy_image_to_buffer(
            self.image,
            ImageLayout::TransferSrcOptimal,
            staging_buffer,
            &[region],
        );

        img.trasition_layout(cmd_buf, self.layout);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);

        let mut buf: Vec<u8> = Vec::with_capacity(size);

        unsafe {
            let src = resources.mem_manager.memory_pool[0].get_ptr(offset as usize);
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

        resources.mem_manager.pop_buffer(base);
    }

    pub fn destroy_view(&mut self, device: &Device) {
        if !self.view.is_null() {
            device.destroy_image_view(self.view, None);
            self.view = ImageView::null()
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        if !self.view.is_null() {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            self.view = ImageView::null();
            self.image = Image::null();
        }
    }
}
