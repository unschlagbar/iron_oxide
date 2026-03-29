use pyronyx::khr::{
    surface::{SurfaceInstance, SurfacePhysicalDevice},
    swapchain::SwapchainDevice,
};
use pyronyx::vk::{
    self, Extent2D, Framebuffer, ImageView, PresentModeKHR, RenderPass, SurfaceCapabilitiesKHR,
    SurfaceFormatKHR, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainKHR,
};
use winit::dpi::PhysicalSize;

use crate::graphics::VkBase;

pub struct Swapchain {
    pub inner: SwapchainKHR,
    pub surface: SurfaceKHR,
    pub image_views: Vec<ImageView>,
    pub capabilities: SurfaceCapabilitiesKHR,
    pub format: SurfaceFormatKHR,
    pub present_mode: PresentModeKHR,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl Swapchain {
    pub fn new(
        base: &VkBase,
        present_mode: vk::PresentModeKHR,
        surface: SurfaceKHR,
        size: PhysicalSize<u32>,
    ) -> Self {
        let target_format = if cfg!(target_os = "android") {
            vk::Format::R8G8B8A8Unorm
        } else {
            vk::Format::B8G8R8A8Unorm
        };

        let mut capabilities = base
            .physical_device
            .get_surface_capabilities(surface)
            .unwrap();

        let format = base
            .physical_device
            .get_surface_formats(surface)
            .unwrap()
            .into_iter()
            .find(|format| {
                format.format == target_format
                    && format.color_space == vk::ColorSpaceKHR::SrgbNonlinear
            })
            .unwrap();

        let present_mode = base
            .physical_device
            .get_surface_present_modes(surface)
            .unwrap()
            .into_iter()
            .find(|pm| *pm == present_mode)
            .unwrap_or(PresentModeKHR::Fifo);

        let composite_alpha = if capabilities
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::Opaque)
        {
            vk::CompositeAlphaFlagsKHR::Opaque
        } else {
            vk::CompositeAlphaFlagsKHR::Inherit
        };

        // Wayland tells with width = u32::MAY that we can decide the size
        if capabilities.current_extent.width == u32::MAX {
            capabilities.current_extent.width = size.width;
            capabilities.current_extent.height = size.height;
        }

        Self {
            inner: SwapchainKHR::null(),
            surface,
            image_views: Vec::new(),
            capabilities,
            format,
            present_mode,
            composite_alpha,
            framebuffers: Vec::new(),
        }
    }

    pub fn create_framebuffer(
        &mut self,
        base: &VkBase,
        image_extent: Extent2D,
        render_pass: RenderPass,
        attachments: &mut [ImageView],
    ) {
        debug_assert_eq!(attachments[0], ImageView::null());

        if self.framebuffers.len() != self.image_views.len() {
            self.framebuffers = vec![Framebuffer::null(); self.image_views.len()];
        }

        for (frame_buffer, &image_view) in self.framebuffers.iter_mut().zip(&self.image_views) {
            attachments[0] = image_view;
            let main_create_info = vk::FramebufferCreateInfo {
                render_pass,
                attachment_count: attachments.len() as u32,
                attachments: attachments.as_ptr(),
                width: image_extent.width,
                height: image_extent.height,
                layers: 1,
                ..Default::default()
            };

            *frame_buffer = base
                .device
                .create_framebuffer(&main_create_info, None)
                .unwrap();
        }
    }

    pub fn update_caps(&mut self, base: &VkBase, size: PhysicalSize<u32>) {
        self.capabilities = base
            .physical_device
            .get_surface_capabilities(self.surface)
            .unwrap();

        // Wayland tells with width = u32::MAY that we can decide the size
        if self.capabilities.current_extent.width == u32::MAX {
            self.capabilities.current_extent.width = size.width;
            self.capabilities.current_extent.height = size.height;
        }
    }

    pub fn recreate(
        &mut self,
        base: &VkBase,
        render_pass: RenderPass,
        attachments: &mut [ImageView],
    ) {
        let image_extent = self.capabilities.current_extent;

        let min_image_count = if self.capabilities.min_image_count > 0 {
            self.capabilities.min_image_count
        } else {
            self.capabilities.max_image_count
        };

        let create_info = vk::SwapchainCreateInfoKHR {
            surface: self.surface,
            min_image_count,
            image_format: self.format.format,
            image_color_space: self.format.color_space,
            image_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::ColorAttachment,
            image_sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 1,
            queue_family_indices: &base.queue_family_index,
            pre_transform: SurfaceTransformFlagsKHR::Identity,
            composite_alpha: self.composite_alpha,
            present_mode: self.present_mode,
            clipped: vk::FALSE,
            old_swapchain: self.inner,
            ..Default::default()
        };

        for i in 0..self.image_views.len() {
            base.device.destroy_framebuffer(self.framebuffers[i], None);
            base.device.destroy_image_view(self.image_views[i], None);
        }
        let new = base.device.create_swapchain(&create_info, None).unwrap();
        base.device.destroy_swapchain(self.inner, None);
        self.inner = new;

        self.create_image_views(base);
        self.create_framebuffer(base, image_extent, render_pass, attachments);
    }

    fn create_image_views(&mut self, base: &VkBase) {
        let present_images = base.device.get_swapchain_images(self.inner).unwrap();

        if self.image_views.len() != present_images.len() {
            self.image_views = vec![ImageView::null(); present_images.len()];
        }

        for (image, image_view) in present_images.into_iter().zip(&mut self.image_views) {
            let create_info = vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::Type2d,
                format: self.format.format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::Color,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            *image_view = base.device.create_image_view(&create_info, None).unwrap();
        }
    }

    pub fn destroy(&mut self, device: &vk::Device, instance: &vk::Instance) {
        for i in 0..self.image_views.len() {
            device.destroy_framebuffer(self.framebuffers[i], None);
            device.destroy_image_view(self.image_views[i], None);
        }

        device.destroy_swapchain(self.inner, None);
        instance.destroy_surface(self.surface, None);
    }
}
