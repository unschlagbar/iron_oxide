use std::mem::MaybeUninit;

use ash::{
    Device,
    khr::{surface, swapchain},
    vk::{
        self, Extent2D, ImageView, PresentModeKHR, RenderPass, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainKHR,
    },
};
use winit::dpi::PhysicalSize;

use crate::graphics::VkBase;

pub struct Swapchain {
    pub loader: swapchain::Device,
    pub inner: SwapchainKHR,
    pub surface_loader: surface::Instance,
    pub surface: SurfaceKHR,
    pub image_views: Vec<ImageView>,
    pub capabilities: SurfaceCapabilitiesKHR,
    pub format: SurfaceFormatKHR,
    pub present_mode: PresentModeKHR,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl Swapchain {
    pub fn create(
        base: &VkBase,
        present_mode: vk::PresentModeKHR,
        surface_loader: surface::Instance,
        surface: SurfaceKHR,
    ) -> Self {
        let loader = swapchain::Device::new(&base.instance, &base.device);
        let target_format = if cfg!(target_os = "android") {
            vk::Format::R8G8B8A8_UNORM
        } else {
            vk::Format::B8G8R8A8_UNORM
        };

        let (capabilities, format, present_mode) = unsafe {
            (
                surface_loader
                    .get_physical_device_surface_capabilities(base.physical_device, surface)
                    .unwrap(),
                surface_loader
                    .get_physical_device_surface_formats(base.physical_device, surface)
                    .unwrap_unchecked()
                    .into_iter()
                    .find(|format| {
                        format.format == target_format
                            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                    })
                    .unwrap(),
                surface_loader
                    .get_physical_device_surface_present_modes(base.physical_device, surface)
                    .unwrap_unchecked()
                    .into_iter()
                    .find(|pm| *pm == present_mode)
                    .unwrap_or(PresentModeKHR::FIFO),
            )
        };

        let composite_alpha = if capabilities
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::OPAQUE)
        {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        } else {
            vk::CompositeAlphaFlagsKHR::INHERIT
        };

        Self {
            loader,
            inner: SwapchainKHR::null(),
            surface_loader,
            surface,
            image_views: Vec::new(),
            #[allow(invalid_value)]
            #[allow(clippy::uninit_assumed_init)]
            capabilities: unsafe { MaybeUninit::uninit().assume_init() },
            format,
            present_mode,
            composite_alpha,
            framebuffers: Vec::new(),
        }
    }

    pub fn create_framebuffer(
        &mut self,
        base: &VkBase,
        image_extend: Extent2D,
        render_pass: RenderPass,
        attachment: ImageView,
    ) {
        if self.framebuffers.capacity() == 0 {
            self.framebuffers = vec![vk::Framebuffer::null(); self.image_views.len()];
        }
        for i in 0..self.image_views.len() {
            let attachments = [self.image_views[i], attachment];
            let main_create_info = vk::FramebufferCreateInfo {
                render_pass,
                attachment_count: attachments.len() as _,
                p_attachments: attachments.as_ptr(),
                width: image_extend.width,
                height: image_extend.height,
                layers: 1,
                ..Default::default()
            };

            self.framebuffers[i] = unsafe {
                base.device
                    .create_framebuffer(&main_create_info, None)
                    .unwrap()
            };
        }
    }

    pub fn update_caps(&mut self, base: &VkBase, size: PhysicalSize<u32>) {
        unsafe {
            self.capabilities = self
                .surface_loader
                .get_physical_device_surface_capabilities(base.physical_device, self.surface)
                .unwrap();
        }

        // Wayland tells with width == u32::MAY that we can decide the size
        if self.capabilities.current_extent.width == u32::MAX {
            self.capabilities.current_extent.width = size.width;
            self.capabilities.current_extent.height = size.height;
        }
    }

    pub fn recreate(&mut self, base: &VkBase, render_pass: RenderPass, attachment: ImageView) {
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
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 1,
            p_queue_family_indices: &base.queue_family_index,
            pre_transform: SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: self.composite_alpha,
            present_mode: self.present_mode,
            clipped: vk::TRUE,
            old_swapchain: self.inner,
            ..Default::default()
        };

        unsafe {
            for i in 0..self.image_views.len() {
                base.device.destroy_framebuffer(self.framebuffers[i], None);
                base.device.destroy_image_view(self.image_views[i], None);
            }
            let new = self
                .loader
                .create_swapchain(&create_info, None)
                .unwrap_unchecked();
            self.loader.destroy_swapchain(self.inner, None);
            self.inner = new;
        }

        self.create_image_views(base);
        self.create_framebuffer(base, image_extent, render_pass, attachment);
    }

    fn create_image_views(&mut self, base: &VkBase) {
        let present_images = unsafe { self.loader.get_swapchain_images(self.inner).unwrap() };
        if self.image_views.capacity() == 0 {
            self.image_views = vec![vk::ImageView::null(); present_images.len()];
        }

        for (i, image) in present_images.into_iter().enumerate() {
            let create_info = vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: self.format.format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            self.image_views[i] =
                unsafe { base.device.create_image_view(&create_info, None).unwrap() };
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            for i in 0..self.image_views.len() {
                device.destroy_framebuffer(self.framebuffers[i], None);
                device.destroy_image_view(self.image_views[i], None);
            }

            self.loader.destroy_swapchain(self.inner, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
