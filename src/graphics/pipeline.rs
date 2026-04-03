use core::slice;

use crate::graphics::{self, VertexDescription, VkBase};
use pyronyx::vk;

use graphics::create_shader_modul;

#[derive(Debug)]
pub struct Pipeline {
    pub this: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl Pipeline {
    pub fn create_ui<T: VertexDescription>(
        base: &VkBase,
        window_size: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: (&[u8], &[u8]),
    ) -> Self {
        let layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: descriptor_set_layouts.len() as _,
            set_layouts: descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };

        let layout = base
            .device
            .create_pipeline_layout(&layout_info, None)
            .unwrap();

        let vertex_shader_buff = shaders.0;
        let fragment_shader_buff = shaders.1;

        let vertex_shader_module = create_shader_modul(base, vertex_shader_buff);
        let fragment_shader_module = create_shader_modul(base, fragment_shader_buff);

        let window_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: window_size,
        };

        let vertex_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::Vertex,
            module: vertex_shader_module,
            name: c"main".as_ptr(),
            ..Default::default()
        };

        let fragment_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::Fragment,
            module: fragment_shader_module,
            name: c"main".as_ptr(),
            ..Default::default()
        };

        let shader_stage = [vertex_stage_info, fragment_stage_info];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: T::GET_BINDING_DESCRIPTION.len() as _,
            vertex_attribute_description_count: T::GET_ATTRIBUTE_DESCRIPTIONS.len() as _,
            vertex_binding_descriptions: T::GET_BINDING_DESCRIPTION.as_ptr(),
            vertex_attribute_descriptions: T::GET_ATTRIBUTE_DESCRIPTIONS.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TriangleStrip,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let dynamic_states = [vk::DynamicState::Viewport, vk::DynamicState::Scissor];

        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as _,
            dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let view_port = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: window_size.width as _,
            height: window_size.height as _,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let view_ports_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            viewports: &view_port as _,
            scissor_count: 1,
            scissors: &window_rect as _,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::Fill,
            cull_mode: vk::CullModeFlags::None,
            front_face: vk::FrontFace::CounterClockwise,
            depth_bias_enable: vk::FALSE,
            line_width: 1.0,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::Type1,
            min_sample_shading: 1.0,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SrcAlpha,
            dst_color_blend_factor: vk::BlendFactor::OneMinusSrcAlpha,
            color_blend_op: vk::BlendOp::Add,
            alpha_blend_op: vk::BlendOp::Add,
            src_alpha_blend_factor: vk::BlendFactor::SrcAlpha,
            dst_alpha_blend_factor: vk::BlendFactor::OneMinusSrcAlpha,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let main_create_info = vk::GraphicsPipelineCreateInfo {
            stage_count: shader_stage.len() as _,
            stages: shader_stage.as_ptr(),
            vertex_input_state: &vertex_input_info,
            input_assembly_state: &input_assembly,
            viewport_state: &view_ports_state,
            rasterization_state: &rasterizer,
            multisample_state: &multisampling,
            color_blend_state: &color_blending,
            depth_stencil_state: &depth_stencil,
            dynamic_state: &dynamic_state,
            layout,
            render_pass,
            subpass: 0,
            base_pipeline_index: -1,
            ..Default::default()
        };

        let mut pipeline = vk::Pipeline::null();

        base.device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[main_create_info],
                None,
                slice::from_mut(&mut pipeline),
            )
            .unwrap();

        base.device
            .destroy_shader_module(vertex_shader_module, None);
        base.device
            .destroy_shader_module(fragment_shader_module, None);

        Self {
            this: pipeline,
            layout,
        }
    }

    pub fn null() -> Self {
        Self {
            this: vk::Pipeline::null(),
            layout: vk::PipelineLayout::null(),
        }
    }

    pub fn destroy(&self, device: &vk::Device) {
        device.destroy_pipeline(self.this, None);
        device.destroy_pipeline_layout(self.layout, None);
    }

    pub fn create_ui_slang<T: VertexDescription>(
        base: &VkBase,
        window_size: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        shaders: &[u8],
    ) -> Self {
        let layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: descriptor_set_layouts.len() as _,
            set_layouts: descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };

        let layout = base
            .device
            .create_pipeline_layout(&layout_info, None)
            .unwrap();

        let window_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: window_size,
        };
        let module = create_shader_modul(base, shaders);

        let vertex_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::Vertex,
            module,
            name: c"vsmain".as_ptr(),
            ..Default::default()
        };

        let fragment_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::Fragment,
            module,
            name: c"fsmain".as_ptr(),
            ..Default::default()
        };

        let shader_stage = [vertex_stage_info, fragment_stage_info];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: T::GET_BINDING_DESCRIPTION.len() as _,
            vertex_attribute_description_count: T::GET_ATTRIBUTE_DESCRIPTIONS.len() as _,
            vertex_binding_descriptions: T::GET_BINDING_DESCRIPTION.as_ptr(),
            vertex_attribute_descriptions: T::GET_ATTRIBUTE_DESCRIPTIONS.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TriangleStrip,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let dynamic_states = [vk::DynamicState::Viewport, vk::DynamicState::Scissor];

        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as _,
            dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let view_port = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: window_size.width as _,
            height: window_size.height as _,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let view_ports_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            viewports: &view_port as _,
            scissor_count: 1,
            scissors: &window_rect as _,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::Fill,
            cull_mode: vk::CullModeFlags::None,
            front_face: vk::FrontFace::CounterClockwise,
            depth_bias_enable: vk::FALSE,
            line_width: 1.0,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::Type1,
            min_sample_shading: 1.0,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SrcAlpha,
            dst_color_blend_factor: vk::BlendFactor::OneMinusSrcAlpha,
            color_blend_op: vk::BlendOp::Add,
            alpha_blend_op: vk::BlendOp::Add,
            src_alpha_blend_factor: vk::BlendFactor::SrcAlpha,
            dst_alpha_blend_factor: vk::BlendFactor::OneMinusSrcAlpha,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let main_create_info = vk::GraphicsPipelineCreateInfo {
            stage_count: shader_stage.len() as _,
            stages: shader_stage.as_ptr(),
            vertex_input_state: &vertex_input_info,
            input_assembly_state: &input_assembly,
            viewport_state: &view_ports_state,
            rasterization_state: &rasterizer,
            multisample_state: &multisampling,
            color_blend_state: &color_blending,
            depth_stencil_state: &depth_stencil,
            dynamic_state: &dynamic_state,
            layout,
            render_pass,
            subpass: 0,
            base_pipeline_index: -1,
            ..Default::default()
        };

        let mut pipeline = vk::Pipeline::null();

        base.device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[main_create_info],
                None,
                slice::from_mut(&mut pipeline),
            )
            .unwrap();

        base.device.destroy_shader_module(module, None);

        Self {
            this: pipeline,
            layout,
        }
    }
}
