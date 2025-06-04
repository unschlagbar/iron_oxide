use ash::vk;

use super::VkBase;

#[inline(always)]
pub fn create_shader_modul(base: &VkBase, raw_code: &[u8]) -> vk::ShaderModule {

    let create_info = vk::ShaderModuleCreateInfo {
        code_size: raw_code.len(),
        p_code: raw_code.as_ptr() as *const u32,
        ..Default::default()
    };

    unsafe { base.device.create_shader_module(&create_info, None).unwrap() }
}