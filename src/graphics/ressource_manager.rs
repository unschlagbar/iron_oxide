use std::slice;

#[cfg(debug_assertions)]
use std::any::TypeId;

use pyronyx::vk::{
    BorderColor, Buffer, BufferUsageFlags, CommandBuffer, CompareOp, DescriptorBufferInfo,
    DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize,
    DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorType, Device, FALSE,
    Filter, ImageLayout, ImageView, IndexType, PipelineBindPoint, Rect2D, Sampler,
    SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, TRUE, WriteDescriptorSet,
};

use crate::{
    graphics::{DrawBatch, Material, MemManager, TextureAtlas, VertexDescription, VkBase},
    primitives::{Matrix4, Vec2},
    ui::{DrawInfo, materials::MatType},
};

pub const MAX_IMGS: u32 = 3;

#[derive(Debug)]
pub struct Resources {
    pub mem_manager: MemManager,
    pub materials: Vec<Material>,
    pub draw_batches: Vec<DrawBatch>,

    pub desc_pool: DescriptorPool,
    pub ubo_set: DescriptorSet,

    pub sampler: Sampler,
    pub sampler_smooth: Sampler,
    pub texture_atlas: TextureAtlas,
}

impl Resources {
    pub fn new(base: &VkBase) -> Self {
        let mut mem_manager = MemManager::new(base);
        mem_manager.allocate_memory(base, mem_manager.host_visible, 5_000_000, 0);
        mem_manager.allocate_memory(base, mem_manager.device_local, 5_000_000, 1);

        mem_manager.map_memory(base, 0, 0, u64::MAX);

        let pool_sizes = [
            DescriptorPoolSize {
                ty: DescriptorType::UniformBuffer,
                descriptor_count: 1,
            },
            DescriptorPoolSize {
                ty: DescriptorType::CombinedImageSampler,
                descriptor_count: MAX_IMGS,
            },
        ];

        let create_info = DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as _,
            pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_IMGS + 1,
            ..Default::default()
        };

        let desc_pool = base
            .device
            .create_descriptor_pool(&create_info, None)
            .unwrap();

        let create_info = SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::ClampToBorder,
            address_mode_v: SamplerAddressMode::ClampToBorder,
            address_mode_w: SamplerAddressMode::ClampToBorder,
            mip_lod_bias: 0.0,
            anisotropy_enable: FALSE,
            max_anisotropy: 0.0,
            compare_enable: FALSE,
            compare_op: CompareOp::Always,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: TRUE,
            ..Default::default()
        };

        let sampler = base.device.create_sampler(&create_info, None).unwrap();

        let create_info = SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::ClampToBorder,
            address_mode_v: SamplerAddressMode::ClampToBorder,
            address_mode_w: SamplerAddressMode::ClampToBorder,
            mip_lod_bias: 0.0,
            anisotropy_enable: FALSE,
            max_anisotropy: 0.0,
            compare_enable: FALSE,
            compare_op: CompareOp::Always,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: TRUE,
            ..Default::default()
        };

        let sampler_smooth = base.device.create_sampler(&create_info, None).unwrap();
        let texture_atlas = TextureAtlas::new(Vec2::new(256, 256));

        Self {
            mem_manager,
            draw_batches: Vec::new(),
            materials: Vec::new(),

            desc_pool,
            ubo_set: DescriptorSet::null(),

            sampler,
            sampler_smooth,
            texture_atlas,
        }
    }

    pub fn create_desc_sets(
        &mut self,
        device: &Device,
        layouts: &[DescriptorSetLayout],
        layout_mats: &[usize],
        uniform_buffer: Buffer,
        image_view: ImageView,
        msdf_view: ImageView,
        atlas_view: ImageView,
    ) {
        let allocate_info = DescriptorSetAllocateInfo {
            descriptor_pool: self.desc_pool,
            descriptor_set_count: layouts.len() as u32,
            set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let mut sets = vec![DescriptorSet::null(); layouts.len()];
        device
            .allocate_descriptor_sets(&allocate_info, &mut sets)
            .unwrap();

        debug_assert_eq!(sets.len() - 1, layout_mats.len());

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
            sampler: self.sampler_smooth,
            image_view,
            image_layout: ImageLayout::ShaderReadOnlyOptimal,
        };

        let msdf_info = DescriptorImageInfo {
            sampler: self.sampler_smooth,
            image_view: msdf_view,
            image_layout: ImageLayout::ShaderReadOnlyOptimal,
        };

        let atlas_image_info = DescriptorImageInfo {
            sampler: self.sampler_smooth,
            image_view: atlas_view,
            image_layout: ImageLayout::ShaderReadOnlyOptimal,
        };

        let descriptor_writes = [
            WriteDescriptorSet {
                dst_set: sets[0],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::UniformBuffer,
                descriptor_count: 1,
                buffer_info: &buffer_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: sets[1],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::CombinedImageSampler,
                descriptor_count: 1,
                image_info: &image_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: sets[2],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::CombinedImageSampler,
                descriptor_count: 1,
                image_info: &msdf_info,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: sets[3],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: DescriptorType::CombinedImageSampler,
                descriptor_count: 1,
                image_info: &atlas_image_info,
                ..Default::default()
            },
        ];

        device.update_descriptor_sets(&descriptor_writes, &[]);
    }

    pub fn add_mat(&mut self, material: Material) {
        self.materials.push(material);
    }

    pub fn add<T: VertexDescription>(&mut self, mat_type: MatType, to_add: T, info: &DrawInfo) {
        self.add_slice(mat_type, &[to_add], info);
    }

    pub fn batch_data<'a, T: VertexDescription>(
        &'a mut self,
        mat_type: MatType,
        info: &DrawInfo,
    ) -> Batch<'a> {
        #[cfg(debug_assertions)]
        assert_eq!(
            self.materials[mat_type as usize].instance_type,
            TypeId::of::<T>()
        );

        for batch in &mut self.draw_batches {
            if batch.mat_type != mat_type {
                batch.done = true;
            }
        }

        if let Some(idx) = self.draw_batches.iter_mut().position(|b| {
            b.mat_type == mat_type && b.clip == info.clip && (b.z_end >= info.z_index || !b.done)
        }) {
            self.draw_batches[idx].z_end = self.draw_batches[idx].z_end.max(info.z_index);
            let batch = &mut self.draw_batches[idx];
            Batch {
                instance_data: &mut batch.instance_data,
                vertex_data: &mut batch.vertex_data,
                index_data: &mut batch.index_data,
            }
        } else {
            self.draw_batches.push(DrawBatch {
                clip: info.clip,
                mat_type,
                instance_data: Vec::new(),
                vertex_data: Vec::new(),
                index_data: Vec::new(),
                size: 0,
                offset: 0,
                z_index: 0,
                z_end: info.z_index,
                first_index: 0,
                index_count: 0,

                done: false,
            });
            let batch = self.draw_batches.last_mut().unwrap();
            Batch {
                instance_data: &mut batch.instance_data,
                vertex_data: &mut batch.vertex_data,
                index_data: &mut batch.index_data,
            }
        }
    }

    pub fn add_slice<T: VertexDescription>(
        &mut self,
        mat_type: MatType,
        slice: &[T],
        info: &DrawInfo,
    ) {
        #[cfg(debug_assertions)]
        assert_eq!(
            self.materials[mat_type as usize].instance_type,
            TypeId::of::<T>()
        );

        for batch in &mut self.draw_batches {
            if batch.mat_type != mat_type {
                batch.done = true;
            }
        }

        let to_add = slice.as_ptr() as *const u8;
        let other = unsafe { slice::from_raw_parts(to_add, size_of_val(slice)) };

        if let Some(batch) = self.draw_batches.iter_mut().find(|b| {
            b.mat_type == mat_type && b.clip == info.clip && (b.z_end >= info.z_index || !b.done)
        }) {
            batch.z_end = batch.z_end.max(info.z_index);
            batch.instance_data.extend_from_slice(other);
        } else {
            let mut instance_data = Vec::new();
            instance_data.extend_from_slice(other);

            self.draw_batches.push(DrawBatch {
                clip: info.clip,
                mat_type,
                instance_data,
                vertex_data: Vec::new(),
                index_data: Vec::new(),
                size: 0,
                offset: 0,
                z_index: 0,
                z_end: info.z_index,

                done: false,
                first_index: 0,
                index_count: 0,
            });
        }
    }

    pub fn upload(&mut self, base: &VkBase, start: usize) {
        self.mem_manager.destroy_buffers(base, start);

        for (i, mat) in self.materials.iter_mut().enumerate() {
            let stride = mat.stride;
            let mut instance_capacity = 0;
            let mut vertex_capacity = 0;
            let mut index_capacity = 0;

            // 1. Kapazitäten berechnen + instanced-Offsets setzen
            for batch in self
                .draw_batches
                .iter_mut()
                .filter(|x| x.mat_type as usize == i)
            {
                batch.offset = (instance_capacity / stride) as u32;
                batch.size = (batch.instance_data.len() / stride) as u32;

                instance_capacity += batch.instance_data.len();
                vertex_capacity += batch.vertex_data.len();
                index_capacity += batch.index_data.len();
            }

            if instance_capacity == 0 && vertex_capacity == 0 && index_capacity == 0 {
                continue;
            }

            let mut instance_buf = Vec::with_capacity(instance_capacity);
            let mut vertex_buf = Vec::with_capacity(vertex_capacity);
            let mut index_buf = Vec::with_capacity(index_capacity);
            let mut current_vertex_count = 0;

            // 2. Daten sammeln + Indices offsetten
            for batch in self
                .draw_batches
                .iter_mut()
                .filter(|x| x.mat_type as usize == i)
            {
                instance_buf.extend_from_slice(&batch.instance_data);

                let num_vertices_this = if mat.stride == 0 {
                    0
                } else {
                    (batch.vertex_data.len() as u32) / mat.stride as u32
                };

                let index_start = index_buf.len() as u32;
                for &idx in &batch.index_data {
                    index_buf.push(idx + current_vertex_count);
                }
                batch.first_index = index_start;
                batch.index_count = batch.index_data.len() as u32;

                vertex_buf.extend_from_slice(&batch.vertex_data);
                current_vertex_count += num_vertices_this;
            }

            // 3. Instance-Buffer (altes Verhalten)
            if instance_capacity > 0 {
                let (buffer, _) = self.mem_manager.create_buffer(
                    base,
                    0,
                    instance_capacity as u64,
                    BufferUsageFlags::VertexBuffer,
                );
                mat.instance_buffer = buffer;

                let offset = self.mem_manager.buffers.last().unwrap().offset as usize;
                let mem = &self.mem_manager.memory_pool[0];
                let dest = mem.get_ptr(offset);
                unsafe {
                    instance_buf
                        .as_ptr()
                        .copy_to_nonoverlapping(dest, instance_buf.len());
                }
            }

            // 4. Vertex-Buffer
            if vertex_capacity > 0 {
                let (buffer, _) = self.mem_manager.create_buffer(
                    base,
                    0,
                    vertex_capacity as u64,
                    BufferUsageFlags::VertexBuffer,
                );
                mat.vertex_buffer = buffer;

                let offset = self.mem_manager.buffers.last().unwrap().offset as usize;
                let mem = &self.mem_manager.memory_pool[0];
                let dest = mem.get_ptr(offset);
                unsafe {
                    vertex_buf
                        .as_ptr()
                        .copy_to_nonoverlapping(dest, vertex_buf.len());
                }
            }

            // 5. Index-Buffer
            if index_capacity > 0 {
                let (buffer, _) = self.mem_manager.create_buffer(
                    base,
                    0,
                    index_capacity as u64 * 4,
                    BufferUsageFlags::IndexBuffer,
                );
                mat.index_buffer = buffer;

                let offset = self.mem_manager.buffers.last().unwrap().offset as usize;
                let mem = &self.mem_manager.memory_pool[0];
                let dest = mem.get_ptr(offset);
                unsafe {
                    let src = index_buf.as_ptr() as *const u8;
                    let len = index_buf.len() * 4;
                    src.copy_to_nonoverlapping(dest, len);
                }
            }
        }
    }

    pub fn draw(&self, cmd: CommandBuffer, clip: Rect2D) {
        if self.draw_batches.is_empty() {
            return;
        }

        let mut current_scissor: Option<Rect2D> = None;

        for batch in &self.draw_batches {
            let mat = &self.materials[batch.mat_type as usize];

            if mat.desc_set != DescriptorSet::null() {
                cmd.bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    mat.pipeline.layout,
                    1,
                    &[mat.desc_set],
                    &[],
                );
            }
            cmd.bind_pipeline(PipelineBindPoint::Graphics, mat.pipeline.this);

            // ── Scissor sauber managen ─────────────────────────────────────
            let desired_scissor = batch.clip;

            if desired_scissor != current_scissor {
                if let Some(sc) = desired_scissor {
                    cmd.set_scissor(0, &[sc]);
                } else {
                    cmd.set_scissor(0, &[clip]); // volle Viewport zurücksetzen
                }
                current_scissor = desired_scissor;
            }

            // ── INDEXED DRAWING (neu) ─────────────────────────────────────
            if batch.index_count > 0 && mat.index_buffer != Buffer::null() {
                if mat.vertex_buffer != Buffer::null() {
                    cmd.bind_vertex_buffers(0, &[mat.vertex_buffer], &[0]);
                }
                cmd.bind_index_buffer(mat.index_buffer, 0, IndexType::Uint32);
                cmd.draw_indexed(batch.index_count, 1, batch.first_index, 0, 0);
            }
            // ── INSTANCED DRAWING (altes Verhalten) ───────────────────────
            else if mat.instance_buffer != Buffer::null() {
                cmd.bind_vertex_buffers(0, &[mat.instance_buffer], &[0]);
                cmd.draw(4, batch.size, 0, batch.offset);
            }
        }
    }

    pub fn clear_batches(&mut self) {
        self.draw_batches.clear();
    }

    pub fn destroy(&mut self, base: &VkBase) {
        for mat in &mut self.materials {
            mat.destroy(&base.device);
        }

        self.texture_atlas.destroy(&base.device);
        self.mem_manager.destroy(base);

        base.device.destroy_sampler(self.sampler, None);
        base.device.destroy_sampler(self.sampler_smooth, None);
        base.device.destroy_descriptor_pool(self.desc_pool, None);
    }
}

pub struct Batch<'a> {
    instance_data: &'a mut Vec<u8>,
    vertex_data: &'a mut Vec<u8>,
    index_data: &'a mut Vec<u32>,
}

impl<'a> Batch<'a> {
    pub fn reserve_instance(&mut self, additional: usize) {
        self.instance_data.reserve(additional);
    }
    pub fn push_vertex<T: VertexDescription>(&mut self, value: &T) {
        let slice =
            unsafe { slice::from_raw_parts(value as *const T as *const u8, size_of_val(value)) };
        self.vertex_data.extend_from_slice(slice);
    }
    pub fn push_instance<T: VertexDescription>(&mut self, value: &T) {
        let slice =
            unsafe { slice::from_raw_parts(value as *const T as *const u8, size_of_val(value)) };
        self.instance_data.extend_from_slice(slice);
    }
    pub fn push_rect<T: VertexDescription>(&mut self, value: &[T; 4]) {
        let slice =
            unsafe { slice::from_raw_parts(value.as_ptr() as *const u8, size_of_val(value)) };
        self.vertex_data.extend_from_slice(slice);

        let len = self.index_data.len() as u32 / 3;
        self.index_data.extend_from_slice(&[len, len + 1, len + 2]);

        let len = len + 1;
        self.index_data.extend_from_slice(&[len, len + 1, len + 2]);
    }
}
