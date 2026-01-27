use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Seek},
    path::PathBuf,
    vec,
};

use ash::vk::{
    self, CommandPool, Extent3D, Format, ImageCreateInfo, ImageLayout, ImageTiling, ImageUsageFlags,
};
use png::{BitDepth, ColorType, Decoder};

use crate::{
    graphics::{MemManager, VkBase, VulkanImage},
    primitives::Vec2,
};

#[derive(Debug)]
pub struct TextureAtlas {
    pub size: Vec2<u16>,
    pub images: Vec<AtlasImage>,
    pub atlas: Option<VulkanImage>,
}

impl TextureAtlas {
    pub fn new(size: Vec2<u16>) -> Self {
        TextureAtlas {
            size,
            images: Vec::new(),
            atlas: None,
        }
    }

    pub fn get_pngs(&mut self, path: PathBuf) -> Vec<(u32, Decoder<BufReader<File>>, String)> {
        let mut pngs = Vec::new();

        let files = path.read_dir().expect("Couldnt load textures");

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
                    uv_start: Vec2::new(0, 0),
                    uv_size: Vec2::new(0, 0),
                    name,
                });
            }
        }

        pngs.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));
        pngs
    }

    pub fn get_pngs_const<'a>(
        &mut self,
        data: &[(&str, &'a [u8])],
    ) -> Vec<(u32, Decoder<Cursor<&'a [u8]>>, String)> {
        let mut pngs = Vec::with_capacity(data.len());
        self.images.reserve_exact(data.len());

        for &(name, file) in data {
            let mut decoder = Decoder::new(Cursor::new(file));
            let height = decoder.read_header_info().unwrap().height;

            pngs.push((height, decoder, name.to_string()));
            self.images.push(AtlasImage {
                uv_start: Vec2::new(0, 0),
                uv_size: Vec2::new(0, 0),
                name: name.to_string(),
            });
        }

        pngs.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));
        pngs
    }

    pub fn load_directory<R: BufRead + Seek>(
        &mut self,
        data: Vec<(u32, Decoder<R>, String)>,
        base: &VkBase,
        mem_slot: usize,
        cmd_pool: CommandPool,
        mem_manager: &mut MemManager,
    ) {
        let mut start_pos = Vec2::new(0, 0);
        let mut next_row = 0;
        let mut image_data = vec![0; self.size.x as usize * self.size.y as usize * 4];

        for (_, mut png, name) in data {
            let mut size = Vec2::new(0, 0);
            let bytes_per_pixel;

            {
                let header_info = png.read_header_info().unwrap();
                size.x = header_info.width as u16;
                size.y = header_info.height as u16;
                bytes_per_pixel = header_info.bytes_per_pixel();

                debug_assert!(header_info.bit_depth == BitDepth::Eight);
                debug_assert!(header_info.color_type == ColorType::Rgba);
            }

            if size.x + start_pos.x > self.size.x {
                start_pos.x = 0;
                start_pos.y += next_row;
                next_row = 0;
                if start_pos.y + size.y > self.size.y {
                    panic!("Texture atlas is full!");
                }
            }
            let idx = self.images.iter().position(|x| x.name == name).unwrap();
            let entry = &mut self.images[idx];
            entry.uv_start = start_pos;
            entry.uv_size = size;

            let mut info = png.read_info().unwrap();
            let mut buf = vec![0; info.output_buffer_size().unwrap()];
            info.next_frame(&mut buf).unwrap();

            for x in 0..entry.uv_size.x {
                for y in 0..entry.uv_size.y {
                    let flat_idx = (y * size.x + x) as usize * 4;

                    for i in 0..bytes_per_pixel {
                        let px_color = buf[flat_idx + i];
                        let flat_idx =
                            ((start_pos.y + y) * self.size.x + (start_pos.x + x)) as usize * 4 + i;
                        image_data[flat_idx] = px_color;
                    }
                }
            }
            start_pos.x += size.x;
            next_row = size.y.max(next_row);
        }

        let extent = Extent3D {
            width: self.size.x as u32,
            height: self.size.y as u32,
            depth: 1,
        };

        let create_info = ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: Format::R8G8B8A8_UNORM,
            extent,
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
            ..Default::default()
        };

        let mut image = VulkanImage::create(base, &create_info);

        mem_manager.upload_image(
            base,
            mem_slot,
            cmd_pool,
            &mut image,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            &image_data,
        );
        image.create_view(base);

        self.atlas = Some(image);
    }

    pub fn destroy(&self, device: &ash::Device) {
        if let Some(image) = &self.atlas {
            unsafe { device.destroy_image_view(image.view, None) };
        }
    }
}

#[derive(Debug, Clone)]
pub struct AtlasImage {
    pub uv_start: Vec2<u16>,
    pub uv_size: Vec2<u16>,
    pub name: String,
}
