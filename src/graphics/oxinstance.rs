use ash::{
    ext,
    khr::{self, surface},
    prelude::VkResult,
    vk,
};
use std::ffi::{CStr, c_char};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct VkBase {
    pub entry: ash::Entry,
    pub instance: ash::Instance,

    #[cfg(debug_assertions)]
    pub debug_utils: ext::debug_utils::Instance,
    #[cfg(debug_assertions)]
    pub utils_messenger: vk::DebugUtilsMessengerEXT,

    pub capabilities: u32,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,

    pub queue_family_index: u32,
    pub queue: vk::Queue,
}

impl VkBase {
    pub fn create(
        required_capabilities: u32,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> (Self, surface::Instance, vk::SurfaceKHR) {
        #[cfg(feature = "linked")]
        let entry = ash::Entry::linked();

        #[cfg(not(feature = "linked"))]
        let entry = unsafe { ash::Entry::load().unwrap() };

        let instance = Self::create_instance(&entry, display_handle);

        #[cfg(debug_assertions)]
        let debug_utils = ext::debug_utils::Instance::new(&entry, &instance);

        #[cfg(debug_assertions)]
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            pfn_user_callback: Some(vulkan_debug_utils_callback),
            ..Default::default()
        };

        #[cfg(debug_assertions)]
        let utils_messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&create_info, None)
                .unwrap()
        };
        let (physical_device, capabilities) =
            Self::select_physical_device(&instance, required_capabilities);

        let surface_loader = surface::Instance::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
                .unwrap_unchecked()
        };

        let queue_family_index =
            Self::get_queue_family_index(physical_device, &instance, surface, &surface_loader);

        let device = Self::create_logical_device(
            &instance,
            physical_device,
            queue_family_index,
            capabilities,
        );

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        (
            Self {
                entry,
                instance,
                #[cfg(debug_assertions)]
                debug_utils,
                #[cfg(debug_assertions)]
                utils_messenger,
                capabilities,
                physical_device,
                device,
                queue_family_index,
                queue,
            },
            surface_loader,
            surface,
        )
    }

    fn create_instance(entry: &ash::Entry, display_handle: RawDisplayHandle) -> ash::Instance {
        let app_name = c"Home Storage";

        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: std::ptr::null(),
            engine_version: vk::make_api_version(0, 1, 0, 0),
            api_version: vk::make_api_version(0, 1, 3, 289),
            ..Default::default()
        };

        let mut extensions = Vec::with_capacity(3);
        extensions
            .extend_from_slice(ash_window::enumerate_required_extensions(display_handle).unwrap());

        #[cfg(debug_assertions)]
        extensions.push(ext::debug_utils::NAME.as_ptr() as _);

        let layer_names: &[&CStr] = {
            if cfg!(debug_assertions) && cfg!(target_os = "windows") {
                &[
                    c"VK_LAYER_KHRONOS_validation",
                    //c"VK_LAYER_LUNARG_monitor",
                ]
            } else if cfg!(target_os = "android") {
                &[c"VK_LAYER_KHRONOS_validation"]
            } else {
                &[]
            }
        };

        let supported_layers: Vec<vk::LayerProperties> =
            unsafe { entry.enumerate_instance_layer_properties().unwrap() };

        // Layer filtern, die in LAYER_NAMES definiert sind und unterstützt werden
        let active_layers: Vec<*const c_char> = layer_names
            .iter()
            .filter_map(|&layer_name| {
                if supported_layers.iter().any(|prop| {
                    let prop_name = unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) };
                    prop_name == layer_name
                }) {
                    Some(layer_name.as_ptr())
                } else {
                    None
                }
            })
            .collect();

        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_layer_count: active_layers.len() as _,
            enabled_extension_count: extensions.len() as _,
            pp_enabled_layer_names: active_layers.as_ptr(),
            pp_enabled_extension_names: extensions.as_ptr() as _,
            ..Default::default()
        };

        unsafe { entry.create_instance(&create_info, None).unwrap() }
    }

    fn select_physical_device(
        instance: &ash::Instance,
        capabilities: u32,
    ) -> (vk::PhysicalDevice, u32) {
        let devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Bro how do you see this without a GPU?");

        let extension: &[&CStr] = if capabilities != 0 {
            &[
                khr::acceleration_structure::NAME,
                khr::ray_tracing_pipeline::NAME,
                khr::deferred_host_operations::NAME,
            ]
        } else {
            &[]
        };

        for &device in &devices {
            let supported_extensions: Vec<&CStr> = unsafe {
                instance
                    .enumerate_device_extension_properties(device)
                    .unwrap()
                    .into_iter()
                    .map(|ext| CStr::from_ptr(ext.extension_name.as_ptr()))
                    .collect()
            };

            let all_supported = extension.iter().all(|&required| {
                supported_extensions
                    .iter()
                    .any(|&supported| supported == required)
            });

            if all_supported {
                return (device, capabilities);
            }
        }
        println!("Gpu doenst support all extentions!");

        (devices[0], 0)
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
        capabilities: u32,
    ) -> ash::Device {
        let mut raytracing_pipeline_structure_features =
            vk::PhysicalDeviceRayTracingPipelineFeaturesKHR {
                ray_tracing_pipeline: vk::TRUE,
                ..Default::default()
            };

        let mut acceleration_structure_features =
            vk::PhysicalDeviceAccelerationStructureFeaturesKHR {
                acceleration_structure: vk::TRUE, // Aktiviere Beschleunigungsstrukturen
                p_next: &mut raytracing_pipeline_structure_features as *mut _ as *mut _,
                ..Default::default()
            };

        let mut buffer_device_address_features = vk::PhysicalDeviceBufferDeviceAddressFeaturesKHR {
            buffer_device_address: vk::TRUE,
            p_next: &mut acceleration_structure_features as *mut _ as _,
            ..Default::default()
        };

        let features2 = {
            if capabilities != 0 {
                vk::PhysicalDeviceFeatures2 {
                    p_next: &mut buffer_device_address_features as *mut _ as *mut _,
                    features: vk::PhysicalDeviceFeatures {
                        shader_int64: vk::FALSE,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                vk::PhysicalDeviceFeatures2::default()
            }
        };

        let extensions: &[&CStr] = {
            if capabilities != 0 {
                &[
                    khr::swapchain::NAME,
                    ext::descriptor_indexing::NAME,
                    khr::buffer_device_address::NAME,
                ]
            } else {
                &[khr::swapchain::NAME]
            }
        };

        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo {
            queue_family_index,
            p_queue_priorities: queue_priorities.as_ptr(),
            queue_count: queue_priorities.len() as _,
            ..Default::default()
        };

        let queue_create_infos = [queue_create_info];

        let device_create_info = vk::DeviceCreateInfo {
            pp_enabled_extension_names: extensions.as_ptr() as _,
            enabled_extension_count: extensions.len() as _,
            queue_create_info_count: queue_create_infos.len() as _,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            p_next: &features2 as *const _ as *const _,
            ..Default::default()
        };

        unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap()
        }
    }

    fn get_queue_family_index(
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
        surface: vk::SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> u32 {
        unsafe {
            let family_queue =
                instance.get_physical_device_queue_family_properties(physical_device);

            for (i, f) in family_queue.into_iter().enumerate() {
                if f.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    && surface_loader
                        .get_physical_device_surface_support(physical_device, i as u32, surface)
                        .unwrap()
                {
                    return i as u32;
                }
            }

            panic!();
        }
    }

    pub fn queue_submit(&self, submits: &[vk::SubmitInfo<'_>], fence: vk::Fence) -> VkResult<()> {
        unsafe { self.device.queue_submit(self.queue, submits, fence) }
    }
}

#[cfg(debug_assertions)]
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}
