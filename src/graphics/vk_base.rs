#[cfg(debug_assertions)]
use pyronyx::ext::debug_utils::{self, DebugUtilsInstance};
use pyronyx::khr::surface::SurfacePhysicalDevice;
use pyronyx::raw_window_handle::{create_surface, get_required_extensions};
use pyronyx::{
    khr,
    vk::{self},
};
#[cfg(debug_assertions)]
use std::ffi::c_void;
use std::ffi::{CStr, c_char};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle},
    window::Window,
};

pub struct VkBase {
    pub instance: vk::Instance,

    #[cfg(debug_assertions)]
    pub utils_messenger: vk::DebugUtilsMessengerEXT,

    pub capabilities: u32,
    pub physical_device: vk::PhysicalDevice,
    pub device: vk::Device,

    pub queue_family_index: u32,
    pub queue: vk::Queue,
}

impl VkBase {
    pub fn create(
        required_capabilities: u32,
        api_version: u32,
        app_name: &CStr,
        window: &Window,
    ) -> (Self, vk::SurfaceKHR) {
        let display_handle = window.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();

        let instance = Self::create_instance(display_handle, api_version, app_name);

        #[cfg(debug_assertions)]
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::Warning
                | vk::DebugUtilsMessageSeverityFlagsEXT::Error
                | vk::DebugUtilsMessageSeverityFlagsEXT::Info
                | vk::DebugUtilsMessageSeverityFlagsEXT::Verbose,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::General
                | vk::DebugUtilsMessageTypeFlagsEXT::Performance
                | vk::DebugUtilsMessageTypeFlagsEXT::Validation,
            pfn_user_callback: Some(vulkan_debug_utils_callback),
            ..Default::default()
        };

        #[cfg(debug_assertions)]
        let utils_messenger = instance
            .create_debug_utils_messenger(&create_info, None)
            .unwrap();

        let (physical_device, capabilities) =
            Self::select_physical_device(&instance, required_capabilities);

        let surface = create_surface(&instance, display_handle, window_handle).unwrap();

        let queue_family_index = Self::get_queue_family_index(physical_device, surface);

        let device = Self::create_logical_device(
            &instance,
            physical_device,
            queue_family_index,
            capabilities,
        );

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        (
            Self {
                instance,
                #[cfg(debug_assertions)]
                utils_messenger,
                capabilities,
                physical_device,
                device,
                queue_family_index,
                queue,
            },
            surface,
        )
    }

    fn create_instance(
        display_handle: RawDisplayHandle,
        api_version: u32,
        app_name: &CStr,
    ) -> vk::Instance {
        let app_info = vk::ApplicationInfo {
            application_name: app_name.as_ptr(),
            application_version: vk::API_VERSION_1_0,
            engine_name: app_name.as_ptr(),
            engine_version: vk::API_VERSION_1_0,
            api_version,
            ..Default::default()
        };

        let mut extensions = Vec::with_capacity(3);
        extensions.extend_from_slice(&get_required_extensions(display_handle).unwrap());

        #[cfg(debug_assertions)]
        extensions.push(debug_utils::NAME.as_ptr());

        let layer_names: &[&CStr] = {
            if cfg!(debug_assertions) {
                &[c"VK_LAYER_KHRONOS_validation"]
            } else {
                &[]
            }
        };

        let supported_layers = vk::enumerate_instance_layer_properties().unwrap();

        // Layer filtering
        let active_layers: Vec<*const c_char> = layer_names
            .iter()
            .filter_map(|&layer_name| {
                if supported_layers.iter().any(|prop| {
                    let prop_name = unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) };
                    prop_name == layer_name
                }) {
                    Some(layer_name.as_ptr())
                } else {
                    println!("Layer: {:?} not aviable", layer_name);
                    None
                }
            })
            .collect();

        let create_info = vk::InstanceCreateInfo {
            application_info: &app_info,
            enabled_layer_count: active_layers.len() as u32,
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_layer_names: active_layers.as_ptr(),
            pp_enabled_extension_names: extensions.as_ptr(),
            ..Default::default()
        };

        vk::Instance::create(&create_info, None).unwrap()
    }

    fn select_physical_device(
        instance: &vk::Instance,
        capabilities: u32,
    ) -> (vk::PhysicalDevice, u32) {
        let devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Bro how do you see this without a GPU?");

        let _extension: &[&CStr] = if capabilities != 0 {
            &[
                khr::acceleration_structure::NAME,
                khr::ray_tracing_pipeline::NAME,
                khr::deferred_host_operations::NAME,
            ]
        } else {
            &[]
        };

        (devices[0], 0)
    }

    fn create_logical_device(
        instance: &vk::Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
        capabilities: u32,
    ) -> vk::Device {
        let features11 = vk::PhysicalDeviceVulkan11Features {
            shader_draw_parameters: vk::TRUE,
            ..Default::default()
        };

        let extensions: &[&CStr] = {
            if capabilities != 0 {
                &[khr::swapchain::NAME]
            } else {
                &[khr::swapchain::NAME]
            }
        };

        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo {
            queue_family_index,
            queue_priorities: queue_priorities.as_ptr(),
            queue_count: queue_priorities.len() as u32,
            ..Default::default()
        };

        let queue_create_infos = [queue_create_info];

        let create_info = vk::DeviceCreateInfo {
            pp_enabled_extension_names: extensions.as_ptr().cast(),
            enabled_extension_count: extensions.len() as u32,
            queue_create_info_count: queue_create_infos.len() as u32,
            queue_create_infos: queue_create_infos.as_ptr(),
            next: &features11 as *const _ as _,
            ..Default::default()
        };

        physical_device
            .create_device(&create_info, None, instance)
            .unwrap()
    }

    #[allow(unused)]
    fn select_queue(&mut self, surface: vk::SurfaceKHR) {
        let mut queue_family = u32::MAX;
        unsafe {
            let family_queue = self.physical_device.get_queue_family_properties();

            for (i, f) in family_queue.into_iter().enumerate() {
                if f.queue_flags.contains(vk::QueueFlags::Graphics)
                    && self
                        .physical_device
                        .get_surface_support(i as u32, surface)
                        .unwrap()
                {
                    queue_family = i as u32;
                }
            }

            if queue_family != u32::MAX {
                self.queue_family_index = queue_family;
            } else {
                panic!("No queue for surface found!");
            }
        }
    }

    fn get_queue_family_index(physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> u32 {
        let family_queue = physical_device.get_queue_family_properties();

        for (i, f) in family_queue.into_iter().enumerate() {
            if f.queue_flags.contains(vk::QueueFlags::Graphics)
                && physical_device
                    .get_surface_support(i as u32, surface)
                    .unwrap()
            {
                return i as u32;
            }
        }

        panic!();
    }

    pub fn queue_submit(
        &self,
        submits: &[vk::SubmitInfo<'_>],
        fence: vk::Fence,
    ) -> Result<(), vk::vkResult> {
        self.queue.submit(submits, fence)
    }

    pub fn device_wait_idle(&self) {
        self.device.device_wait_idle().unwrap()
    }

    pub fn destroy(&mut self) {
        self.device.destroy_device(None);
        #[cfg(debug_assertions)]
        self.instance
            .destroy_debug_utils_messenger(self.utils_messenger, None);
        self.instance.destroy(None)
    }
}

#[cfg(debug_assertions)]
extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> u32 {
    let message = unsafe { CStr::from_ptr((*callback_data).message) };
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    0
}
