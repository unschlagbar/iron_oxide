use std::{
    fs::{self, File},
    io::BufReader,
    ptr, vec,
};

use ash::vk::{
    self, BufferUsageFlags, CommandPool, Extent3D, Format, ImageTiling, ImageUsageFlags,
    MemoryPropertyFlags,
};
use png::{BitDepth, ColorType, Decoder};

use crate::graphics::{Buffer, Image, SinlgeTimeCommands, VkBase};

#[derive(Debug)]
pub struct TextureAtlas {
    pub size: (u16, u16),
    pub images: Vec<AtlasImage>,
    pub atlas: Option<Image>,
}

impl TextureAtlas {
    pub fn new(size: (u16, u16)) -> Self {
        TextureAtlas {
            size,
            images: Vec::new(),
            atlas: None,
        }
    }

    pub fn load_directory(&mut self, path: &str, base: &VkBase, cmd_pool: CommandPool) {
        let files = if let Ok(dir) = fs::read_dir(path) {
            dir
        } else {
            println!("Couldnt load textures");
            return;
        };

        let mut pngs = Vec::new();

        for file in files {
            let file = file.unwrap();
            let path = file.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("png") {
                let file = File::open(&path).unwrap();

                let mut decoder = Decoder::new(BufReader::new(file));
                let height = decoder.read_header_info().unwrap().height;
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();

                pngs.push((height, decoder, name.clone()));
                self.images.push(AtlasImage {
                    uv_start: (0, 0),
                    uv_size: (0, 0),
                    name,
                });
            }
        }

        pngs.sort_by(|(a, _, _), (b, _, _)| b.cmp(&a));
        let mut start_pos: (u16, u16) = (0, 0);
        let mut image_data = vec![0; self.size.0 as usize * self.size.1 as usize * 4];

        for (_, mut png, name) in pngs {
            let width;

            let height;
            let bytes_per_pixel;

            {
                let header_info = png.read_header_info().unwrap();
                width = header_info.width as u16;
                height = header_info.height as u16;
                bytes_per_pixel = header_info.bytes_per_pixel();

                assert!(header_info.bit_depth == BitDepth::Eight);
                assert!(header_info.color_type == ColorType::Rgba);
            }

            if width + start_pos.0 > self.size.0 {
                start_pos.0 = 0;
                start_pos.1 += height;
                if start_pos.1 + height > self.size.1 {
                    panic!("Texture atlas is full!");
                }
            }
            let idx = self.images.iter().position(|x| x.name == name).unwrap();
            let entry = &mut self.images[idx];
            entry.uv_start = start_pos;
            entry.uv_size = (width, height);

            let mut info = png.read_info().unwrap();
            let mut buf = vec![0; info.output_buffer_size().unwrap()];
            info.next_frame(&mut buf).unwrap();

            for x in 0..entry.uv_size.0 {
                for y in 0..entry.uv_size.1 {
                    let flat_idx = (y * width + x) as usize * 4;

                    for i in 0..bytes_per_pixel {
                        let px_color = buf[flat_idx + i];
                        let flat_idx =
                            ((start_pos.1 + y) * self.size.0 + (start_pos.0 + x)) as usize * 4 + i;
                        image_data[flat_idx] = px_color;
                    }
                }
            }
            start_pos.0 += width;
        }

        let size = image_data.len() as u64;
        let extent = Extent3D {
            width: self.size.0 as _,
            height: self.size.1 as _,
            depth: 1,
        };
        let cmd_buf = SinlgeTimeCommands::begin(&base, cmd_pool);

        let mut staging_buffer = Buffer::create(
            base,
            size,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let mapped_memory = staging_buffer.map_memory(&base.device, size, 0);
        unsafe {
            ptr::copy_nonoverlapping(image_data.as_ptr(), mapped_memory, image_data.len());
        };
        staging_buffer.unmap_memory(&base.device);

        let mut atlas = Image::create(
            base,
            extent,
            Format::R8G8B8A8_UNORM,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );

        atlas.trasition_layout(base, cmd_buf, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        atlas.copy_from_buffer(
            base,
            cmd_buf,
            &staging_buffer,
            extent,
            vk::ImageAspectFlags::COLOR,
        );
        atlas.trasition_layout(base, cmd_buf, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        SinlgeTimeCommands::end(base, cmd_pool, cmd_buf);
        staging_buffer.destroy(&base.device);
        atlas.create_view(base, vk::ImageAspectFlags::COLOR);

        self.atlas = Some(atlas);
    }

    pub fn destroy(&self, device: &ash::Device) {
        if let Some(atlas) = &self.atlas {
            atlas.destroy(device);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AtlasImage {
    pub uv_start: (u16, u16),
    pub uv_size: (u16, u16),
    pub name: String,
}
