fn create_texture_image(
        base: &VkBase,
        cmd_buf: vk::CommandBuffer,
    ) -> (graphics::Image, Buffer) {
        let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
            "../textures/texture.png"
        )));

        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let width = info.width;
        let height = info.height;
        let image_size = buf.len() as u64;
        let extent = Extent3D {
            width,
            height,
            depth: 1,
        };

        let staging_buffer = Buffer::create(
            base,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let mapped_memory = staging_buffer.map_memory(&base.device, image_size, 0);
        unsafe {
            copy_nonoverlapping(buf.as_ptr(), mapped_memory, image_size as usize);
        };
        staging_buffer.unmap_memory(&base.device);

        let mut texture_image = graphics::Image::create(
            base,
            extent,
            Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );

        texture_image.trasition_layout(base, cmd_buf, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        texture_image.copy_from_buffer(
            base,
            cmd_buf,
            &staging_buffer,
            extent,
            vk::ImageAspectFlags::COLOR,
        );
        texture_image.trasition_layout(base, cmd_buf, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        (texture_image, staging_buffer)
    }