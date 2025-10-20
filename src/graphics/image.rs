use std::{
    fmt::{Debug, Formatter},
    fs::File,
    io::BufWriter,
};

use ash::vk::{self, ImageView};
use png::Encoder;

use crate::graphics::SinlgeTimeCommands;

use super::{Buffer, VkBase};

#[derive(Clone)]
pub struct Image {
    pub inner: vk::Image,
    pub mem: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub layout: vk::ImageLayout,
}

impl Image {
    pub fn create(
        base: &VkBase,
        extent: vk::Extent3D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        let image = {
            let create_info = vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent,
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                ..Default::default()
            };
            unsafe { base.device.create_image(&create_info, None).unwrap() }
        };

        let memory_requirements = unsafe { base.device.get_image_memory_requirements(image) };

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: super::buffer::find_memory_type(
                &base,
                memory_requirements.memory_type_bits,
                properties,
            ),
            ..Default::default()
        };

        let mem = unsafe { base.device.allocate_memory(&allocate_info, None).unwrap() };
        unsafe { base.device.bind_image_memory(image, mem, 0).unwrap() };
        Self {
            inner: image,
            mem,
            view: ImageView::null(),
            format,
            layout: vk::ImageLayout::UNDEFINED,
        }
    }

    pub fn create_view(&mut self, base: &VkBase, aspect_flags: vk::ImageAspectFlags) {
        let create_info = vk::ImageViewCreateInfo {
            image: self.inner,
            view_type: vk::ImageViewType::TYPE_2D,
            format: self.format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        self.view = unsafe { base.device.create_image_view(&create_info, None).unwrap() }
    }

    pub fn trasition_layout(
        &mut self,
        base: &VkBase,
        cmd_buf: vk::CommandBuffer,
        new_layout: vk::ImageLayout,
    ) {
        let mut barrier = vk::ImageMemoryBarrier {
            old_layout: self.layout,
            new_layout,
            image: self.inner,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: {
                    if self.format == vk::Format::D32_SFLOAT || self.format == vk::Format::D16_UNORM
                    {
                        vk::ImageAspectFlags::DEPTH
                    } else if self.format == vk::Format::D24_UNORM_S8_UINT {
                        vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
                    } else {
                        vk::ImageAspectFlags::COLOR
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
        if self.layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::NONE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;

            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } else if self.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::TRANSFER_SRC_OPTIMAL
            && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::GENERAL
        {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::TRANSFER_SRC_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::NONE;
            barrier.dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;

            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
        } else if self.layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            && new_layout == vk::ImageLayout::TRANSFER_SRC_OPTIMAL
        {
            barrier.src_access_mask = vk::AccessFlags::SHADER_READ;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

            source_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } else {
            panic!(
                "From layout: {:?} to layout: {:?} is not implemented!",
                self.layout, new_layout
            );
        }
        self.layout = new_layout;
        unsafe {
            base.device.cmd_pipeline_barrier(
                cmd_buf,
                source_stage,
                destination_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        }
    }

    pub fn copy_from_buffer(
        &self,
        base: &VkBase,
        cmd_buf: vk::CommandBuffer,
        buffer: &Buffer,
        extent: vk::Extent3D,
        aspect_mask: vk::ImageAspectFlags,
    ) {
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: extent,
        };

        unsafe {
            base.device.cmd_copy_buffer_to_image(
                cmd_buf,
                buffer.inner,
                self.inner,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            )
        };
    }

    pub fn save_to_png(
        &self,
        base: &VkBase,
        cmd_pool: vk::CommandPool,
        extent: vk::Extent3D,
        path: &str,
    ) {
        let image_size = extent.width as u64 * extent.height as u64 * 4;

        let mut staging_buffer = Buffer::create(
            base,
            image_size,
            vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);

        let mut img = self.clone();

        img.trasition_layout(base, cmd_buf, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: extent,
        };
        unsafe {
            base.device.cmd_copy_image_to_buffer(
                cmd_buf,
                self.inner,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                staging_buffer.inner,
                &[region],
            );
        }

        let cmd_buf = SinlgeTimeCommands::begin(base, cmd_pool);
        img.trasition_layout(base, cmd_buf, self.layout);
        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);

        let data_ptr = unsafe {
            base.device
                .map_memory(
                    staging_buffer.mem,
                    0,
                    image_size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap()
        };
        let data =
            unsafe { std::slice::from_raw_parts(data_ptr as *const u8, image_size as usize) };
        unsafe {
            base.device.unmap_memory(staging_buffer.mem);
        }

        // 6. PNG schreiben
        let file = File::create(path).expect("Failed to create PNG file");
        let mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(&mut w, extent.width, extent.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();

        staging_buffer.destroy(&base.device);
    }

    #[inline]
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            if self.view != ImageView::null() {
                device.destroy_image_view(self.view, None);
            }
            device.destroy_image(self.inner, None);
            device.free_memory(self.mem, None);
        }
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Image {{ format: {:?}, layout: {:?} }}",
            self.format, self.layout
        )
    }
}
