use ash::vk;
use graphics::FontVertex;
use winit::dpi::PhysicalSize;
use crate::graphics::{self, VkBase};

use graphics::create_shader_modul;

pub fn font_pipeline(base: &VkBase, window_size: PhysicalSize<u32>, render_pass: vk::RenderPass, descriptor_set_layout: vk::DescriptorSetLayout, shaders: (&[u8], &[u8])) -> (vk::PipelineLayout, vk::Pipeline) {
    let vertex_shader_buff = shaders.0;
    let fragment_shader_buff = shaders.1;

    let vertex_shader_module = create_shader_modul(base, vertex_shader_buff);
    let fragment_shader_module = create_shader_modul(base, fragment_shader_buff);

    let window_rect = vk::Rect2D { 
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk::Extent2D { width: window_size.width, height: window_size.height },
    };

    let vertex_stage_info = vk::PipelineShaderStageCreateInfo {
        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        stage: vk::ShaderStageFlags::VERTEX,
        module: vertex_shader_module,
        p_name:  c"main".as_ptr(),
        ..Default::default()
    };

    let fragment_stage_info = vk::PipelineShaderStageCreateInfo {
        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        stage: vk::ShaderStageFlags::FRAGMENT,
        module: fragment_shader_module,
        p_name:  c"main".as_ptr(),
        ..Default::default()
    };

    let shader_stage = [vertex_stage_info, fragment_stage_info];

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
        vertex_binding_description_count: FontVertex::GET_BINDING_DESCRIPTION.len() as _,
        vertex_attribute_description_count: FontVertex::GET_ATTRIBUTE_DESCRIPTIONS.len() as _,
        p_vertex_binding_descriptions: FontVertex::GET_BINDING_DESCRIPTION.as_ptr(),
        p_vertex_attribute_descriptions: FontVertex::GET_ATTRIBUTE_DESCRIPTIONS.as_ptr(),
        ..Default::default()
    };

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_STRIP,
        primitive_restart_enable: vk::FALSE,
        ..Default::default()
    };

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    
    let dynamic_state = vk::PipelineDynamicStateCreateInfo {
        dynamic_state_count: dynamic_states.len() as _,
        p_dynamic_states: dynamic_states.as_ptr(),
        ..Default::default()
    };

    let view_port = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: window_size.width as _,
        height: window_size.height as _,
        min_depth: 0.0,
        max_depth: 1.0
    };

    let view_ports_state = vk::PipelineViewportStateCreateInfo {
        viewport_count: 1,
        p_viewports: &view_port as _,
        scissor_count: 1,
        p_scissors: &window_rect as _,
        ..Default::default()
    };

    let rasterizer = vk::PipelineRasterizationStateCreateInfo {
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::FRONT,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        depth_bias_enable: vk::TRUE,
        line_width: 1.0,
        depth_bias_constant_factor: 0.5,
        ..Default::default()
    };

    let multisampling = vk::PipelineMultisampleStateCreateInfo {
        sample_shading_enable: vk::FALSE,
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        min_sample_shading: 1.0,
        ..Default::default()
    };

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        //src_alpha_blend_factor: vk::BlendFactor::ONE,
        //dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
        ..Default::default()
    };

    let color_blending = vk::PipelineColorBlendStateCreateInfo {
        logic_op_enable: vk::FALSE,
        logic_op: vk::LogicOp::COPY,
        attachment_count: 1,
        p_attachments: &color_blend_attachment,
        blend_constants: [0.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };

    let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
        set_layout_count: 1,
        p_set_layouts: &descriptor_set_layout,
        ..Default::default()
    };

    let pipeline_layout = unsafe { base.device.create_pipeline_layout(&pipeline_layout_info, None).unwrap() };

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::FALSE,
        depth_write_enable: vk::FALSE,
        depth_compare_op: vk::CompareOp::LESS,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        ..Default::default()
    };

    let main_create_info = vk::GraphicsPipelineCreateInfo {
        stage_count: 2,
        p_stages: shader_stage.as_ptr(),
        p_vertex_input_state: &vertex_input_info,
        p_input_assembly_state: &input_assembly,
        p_viewport_state: &view_ports_state,
        p_rasterization_state: &rasterizer,
        p_multisample_state: &multisampling,
        p_color_blend_state: &color_blending,
        p_depth_stencil_state: &depth_stencil,
        p_dynamic_state: &dynamic_state,
        layout: pipeline_layout,
        render_pass,
        subpass: 1,
        base_pipeline_index: -1,
        ..Default::default()
    };

    let pipelines = unsafe { base.device.create_graphics_pipelines(vk::PipelineCache::null(), &[main_create_info], None).unwrap()[0] };

    unsafe {
        base.device.destroy_shader_module(vertex_shader_module, None);
        base.device.destroy_shader_module(fragment_shader_module, None);
    }

    (pipeline_layout, pipelines)
}   