use std::{any::TypeId, slice};

use ash::vk::{
    BorderColor, Buffer, BufferUsageFlags, CommandBuffer, CompareOp, DescriptorBufferInfo,
    DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize,
    DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorType, FALSE, Filter,
    ImageLayout, ImageView, PipelineBindPoint, Rect2D, Sampler, SamplerAddressMode,
    SamplerCreateInfo, SamplerMipmapMode, TRUE, WriteDescriptorSet,
};

use crate::{
    graphics::{BufferManager, DrawBatch, Material, TextureAtlas, VertexDescription, VkBase},
    primitives::Matrix4,
    ui::materials::MatType,
};

pub const MAX_IMGS: u32 = 2;

#[derive(Debug)]
pub struct Ressources {
    pub buffer_manager: BufferManager,
    pub materials: Vec<Material>,
    pub draw_batches: Vec<Vec<DrawBatch>>,

    pub desc_pool: DescriptorPool,
    pub ubo_set: DescriptorSet,

    pub sampler: Sampler,
    pub texture_atlas: TextureAtlas,
}

impl Ressources {
    pub fn new(base: &VkBase, atlas: TextureAtlas) -> Self {
        let mut buffer_manager = BufferManager::new(base);
        buffer_manager.allocate_memory(base, buffer_manager.host_visible, 500_000);

        let pool_sizes = [
            DescriptorPoolSize {
                ty: DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
            DescriptorPoolSize {
                ty: DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_IMGS,
            },
        ];

        let create_info = DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as _,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_IMGS + 1,
            ..Default::default()
        };

        let desc_pool = unsafe {
            base.device
                .create_descriptor_pool(&create_info, None)
                .unwrap()
        };

        let create_info = SamplerCreateInfo {
            mag_filter: Filter::NEAREST,
            min_filter: Filter::NEAREST,
            mipmap_mode: SamplerMipmapMode::NEAREST,
            address_mode_u: SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: SamplerAddressMode::CLAMP_TO_BORDER,
            mip_lod_bias: 0.0,
            anisotropy_enable: FALSE,
            max_anisotropy: 0.0,
            compare_enable: FALSE,
            compare_op: CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: BorderColor::FLOAT_TRANSPARENT_BLACK,
            unnormalized_coordinates: TRUE,
            ..Default::default()
        };

        let sampler = unsafe { base.device.create_sampler(&create_info, None).unwrap() };

        Self {
            buffer_manager,
            draw_batches: Vec::new(),
            materials: Vec::new(),

            desc_pool,
            ubo_set: DescriptorSet::null(),

            sampler,
            texture_atlas: atlas,
        }
    }

    pub fn create_desc_sets(
        &mut self,
        device: &ash::Device,
        layouts: &[DescriptorSetLayout],
        layout_mats: &[usize],
        uniform_buffer: Buffer,
        image_view: ImageView,
        atlas_view: ImageView,
    ) {
        let allocate_info = DescriptorSetAllocateInfo {
            descriptor_pool: self.desc_pool,
            descriptor_set_count: layouts.len() as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let sets = unsafe { device.allocate_descriptor_sets(&allocate_info).unwrap() };

        assert_eq!(sets.len() - 1, layout_mats.len());

        self.ubo_set = sets[0];

        for (&mat_i, set) in layout_mats.iter().zip(&sets[1..]) {
            self.materials[mat_i].desc_set = *set;
        }

        let buffer_info = DescriptorBufferInfo {
            buffer: uniform_buffer,
            offset: 0,
            range: size_of::<Matrix4>() as u64,
        };

        let image_info = DescriptorImageInfo {
            sampler: self.sampler,
            image_view,
            image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let atlas_image_info = DescriptorImageInfo {
            sampler: self.sampler,
            image_view: atlas_view,
            image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let descriptor_writes = [
            WriteDescriptorSet {
                dst_set: sets[0],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: sets[1],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &image_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: sets[2],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &atlas_image_info,
                ..Default::default()
            },
        ];

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    pub fn add_mat(&mut self, material: Material) {
        self.materials.push(material);
        self.draw_batches.push(Vec::new());
    }

    pub fn add<T: VertexDescription>(
        &mut self,
        mat_type: MatType,
        to_add: &T,
        clip: Option<Rect2D>,
    ) {
        let material = &self.materials[mat_type as usize];
        assert_eq!(material.instance_type, TypeId::of::<T>());

        let mat_batch = &mut self.draw_batches[mat_type as usize];
        let to_add = to_add as *const T as *const u8;
        let other = unsafe { slice::from_raw_parts(to_add, size_of::<T>()) };

        if let Some(group) = mat_batch.iter_mut().find(|x| x.clip == clip) {
            group.data.extend_from_slice(other);
        } else {
            let mut data = Vec::new();
            data.extend_from_slice(other);

            mat_batch.push(DrawBatch {
                //desc,
                clip,
                data,
                size: 0,
                offset: 0,
            });
        }
    }

    pub fn draw(&self, device: &ash::Device, cmd: CommandBuffer, clip: Rect2D) {
        if self.draw_batches.is_empty() {
            return;
        }

        for (i, mat) in self.materials.iter().enumerate() {
            let mut last_had_clip = false;

            let batches = &self.draw_batches[i];

            if batches.is_empty() {
                continue;
            }

            unsafe {
                if mat.desc_set != DescriptorSet::null() {
                    device.cmd_bind_descriptor_sets(
                        cmd,
                        PipelineBindPoint::GRAPHICS,
                        mat.pipeline.layout,
                        1,
                        &[mat.desc_set],
                        &[],
                    );
                }
                device.cmd_bind_pipeline(cmd, PipelineBindPoint::GRAPHICS, mat.pipeline.this);
                device.cmd_bind_vertex_buffers(cmd, 0, &[mat.buffer], &[0]);

                for batch in batches {
                    if let Some(clip) = batch.clip {
                        device.cmd_set_scissor(cmd, 0, &[clip]);
                        last_had_clip = true;
                    } else if last_had_clip {
                        device.cmd_set_scissor(cmd, 0, &[clip]);
                        last_had_clip = false;
                    }
                    device.cmd_draw(cmd, 4, batch.size, 0, batch.offset);
                }

                if last_had_clip {
                    device.cmd_set_scissor(cmd, 0, &[clip]);
                }
            }
        }
    }

    pub fn clear_batches(&mut self) {
        for batch in &mut self.draw_batches {
            batch.clear();
        }
    }

    pub fn upload(&mut self, base: &VkBase, start: usize) {
        self.buffer_manager.destroy_buffers(base, start);

        for (i, mat) in self.materials.iter_mut().enumerate() {
            if self.draw_batches[i].is_empty() {
                continue;
            }

            let stride = mat.stride as u32;
            let mut capacity = 0;

            for batch in &mut self.draw_batches[i] {
                batch.offset = capacity / stride;
                batch.size = batch.data.len() as u32 / stride;

                capacity += batch.data.len() as u32;
            }

            let mut buf = Vec::with_capacity(capacity as usize);

            for batch in &mut self.draw_batches[i] {
                buf.extend_from_slice(&batch.data);
            }

            let (buffer, buffer_size) = self.buffer_manager.create_buffer(
                base,
                buf.len() as u64,
                BufferUsageFlags::VERTEX_BUFFER,
            );

            mat.buffer = buffer;
            mat.buffer_size = buffer_size;

            let offset = self.buffer_manager.buffers.last().unwrap().1 as usize;

            let mem = &self.buffer_manager.memory_pool[0];
            let dest = mem.get_ptr(offset);

            unsafe {
                buf.as_ptr().copy_to_nonoverlapping(dest, buf.len());
            };
        }
    }

    pub fn destroy(&mut self, base: &VkBase) {
        for mat in &mut self.materials {
            mat.destroy(&base.device);
        }

        self.buffer_manager.destroy(base);
        self.texture_atlas.destroy(&base.device);

        unsafe {
            base.device.destroy_sampler(self.sampler, None);
            base.device.destroy_descriptor_pool(self.desc_pool, None);
        }
    }
}
