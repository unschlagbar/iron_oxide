use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    ptr, vec,
};

use ash::vk::{
    self, BufferUsageFlags, CommandPool, Extent3D, Format, ImageTiling, ImageUsageFlags,
    MemoryPropertyFlags,
};
use png::{BitDepth, Decoder};

use crate::graphics::{Buffer, Image, SinlgeTimeCommands, VkBase};

#[derive(Debug)]
pub struct TextureAtlas {
    pub size: (u32, u32),
    pub images: HashMap<String, (u32, u32, u32, u32)>,
    pub atlas: Option<Image>,
}

impl TextureAtlas {
    pub fn new(size: (u32, u32)) -> Self {
        TextureAtlas {
            size,
            images: HashMap::new(),
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

                pngs.push((height, decoder, name));
            }
        }

        pngs.sort_unstable_by(|(a, _, _), (b, _, _)| b.cmp(&a));
        let mut start_pos = (0, 0);
        let mut image_data = vec![0.0; self.size.0 as usize * self.size.1 as usize * 4];

        for (_, mut png, name) in pngs {
            let width;
            let height;
            let bytes_per_pixel;

            {
                let header_info = png.read_header_info().unwrap();
                width = header_info.width;
                height = header_info.height;
                bytes_per_pixel = header_info.bytes_per_pixel();
                assert!(header_info.bit_depth == BitDepth::Eight);
            }

            if width + start_pos.0 > self.size.0 {
                start_pos.0 = 0;
                start_pos.1 += height;
                if start_pos.1 + height > self.size.1 {
                    panic!("Texture atlas is full!");
                }
            }
            let uv = (start_pos.0, start_pos.1, width, height);
            self.images.insert(name, uv);
            start_pos.0 += width;

            let mut info = png.read_info().unwrap();
            let mut buf = vec![0; info.output_buffer_size().unwrap()];
            info.next_frame(&mut buf).unwrap();

            for x in 0..uv.2 {
                for y in 0..uv.3 {
                    let flat_idx = (y * width + x) as usize * 4;

                    for i in 0..bytes_per_pixel {
                        let px_color = buf[flat_idx + i];
                        let flat_idx =
                            ((start_pos.1 + y) * self.size.0 + (start_pos.0 + x)) as usize * 4 + i;
                        image_data[flat_idx] = px_color as f32 / 255.0;
                    }
                }
            }
        }

        let size = image_data.len() as u64 * 4;
        let extent = Extent3D {
            width: self.size.0,
            height: self.size.1,
            depth: 1,
        };
        let cmd_buf = SinlgeTimeCommands::begin(&base, cmd_pool);

        let staging_buffer = Buffer::create(
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
            Format::R8G8B8A8_SRGB,
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

        println!("{:?}", self.images)
    }

    pub fn destroy(&self, device: &ash::Device) {
        if let Some(atlas) = &self.atlas {
            atlas.destroy(device);
        }
    }
}
