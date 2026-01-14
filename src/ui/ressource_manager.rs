use std::{any::TypeId, slice};

use ash::vk::{
    BufferUsageFlags, CommandBuffer, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool,
    DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
    DescriptorSetLayout, DescriptorType, ImageLayout, ImageView, MemoryMapFlags, PipelineBindPoint,
    Rect2D, Sampler, WriteDescriptorSet,
};

use crate::{
    graphics::{self, BufferManager, TextureAtlas, VertexDescription, VkBase},
    primitives::Matrix4,
    ui::materials::{DrawBatch, MatType, Material},
};

pub const MAX_IMGS: u32 = 2;

#[derive(Debug)]
pub struct Ressources {
    pub buffer_manager: BufferManager,
    pub materials: Vec<Material>,
    pub draw_batches: Vec<Vec<DrawBatch>>,

    pub desc_pool: DescriptorPool,
    pub ubo_set: DescriptorSet,
    pub img_set: DescriptorSet,
    pub atl_set: DescriptorSet,

    pub texture_atlas: TextureAtlas,
}

impl Ressources {
    pub fn new(base: &VkBase, atlas: TextureAtlas) -> Self {
        let mut buffer_manager = BufferManager::new(base);
        buffer_manager.allocate_memory(base, buffer_manager.host_visible, 5_000_000);

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

        let pool_info = DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as _,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_IMGS + 1,
            ..Default::default()
        };

        let desc_pool = unsafe {
            base.device
                .create_descriptor_pool(&pool_info, None)
                .unwrap()
        };

        Self {
            buffer_manager,
            draw_batches: Vec::new(),
            materials: Vec::new(),

            desc_pool,
            ubo_set: DescriptorSet::null(),
            img_set: DescriptorSet::null(),
            atl_set: DescriptorSet::null(),

            texture_atlas: atlas,
        }
    }

    pub fn create_desc_sets(
        &mut self,
        device: &ash::Device,
        layouts: &[DescriptorSetLayout],
        uniform_buffer: &graphics::Buffer,
        image_view: ImageView,
        atlas_view: ImageView,
        sampler: Sampler,
    ) {
        let allocate_info = DescriptorSetAllocateInfo {
            descriptor_pool: self.desc_pool,
            descriptor_set_count: layouts.len() as _,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let mut sets = unsafe {
            device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()
                .into_iter()
        };

        let ubo_set = sets.next().unwrap();
        let img_set = sets.next().unwrap();
        let atl_set = sets.next().unwrap();

        let buffer_info = DescriptorBufferInfo {
            buffer: uniform_buffer.inner,
            offset: 0,
            range: size_of::<Matrix4>() as u64,
        };

        let image_info = DescriptorImageInfo {
            sampler,
            image_view,
            image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let atlas_image_info = DescriptorImageInfo {
            sampler,
            image_view: atlas_view,
            image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let descriptor_writes = [
            WriteDescriptorSet {
                dst_set: ubo_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: img_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &image_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: atl_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                p_image_info: &atlas_image_info,
                ..Default::default()
            },
        ];

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };

        self.ubo_set = ubo_set;
        self.img_set = img_set;
        self.atl_set = atl_set;
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

    pub fn upload(&mut self, base: &VkBase) {
        self.buffer_manager.destroy_buffers(base);

        for (i, mat) in self.materials.iter_mut().enumerate() {
            let mut buf = Vec::new();
            let stride = mat.stride as u32;

            for batch in &mut self.draw_batches[i] {
                batch.offset = buf.len() as u32 / stride;
                batch.size = batch.data.len() as u32 / stride;

                buf.extend_from_slice(&batch.data);
            }

            if buf.is_empty() {
                continue;
            }

            let offset = self.buffer_manager.memory_pool[0].used;

            let (buffer, buffer_size) = self.buffer_manager.create_buffer(
                base,
                buf.len() as u64,
                BufferUsageFlags::VERTEX_BUFFER,
            );
            mat.buffer = buffer;
            mat.buffer_size = buffer_size;

            let mem = &self.buffer_manager.memory_pool[0];

            let mapped_memory: *mut u8 = unsafe {
                base.device
                    .map_memory(
                        mem.memory,
                        offset,
                        buf.len() as u64,
                        MemoryMapFlags::empty(),
                    )
                    .unwrap()
                    .cast()
            };
            unsafe {
                buf.as_ptr()
                    .copy_to_nonoverlapping(mapped_memory, buf.len());
                base.device.unmap_memory(mem.memory);
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
            base.device.destroy_descriptor_pool(self.desc_pool, None);
        }
    }
}
