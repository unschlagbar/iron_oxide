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

    pub physical_device: vk::PhysicalDevice,
    pub device: vk::Device,

    pub graphics_family: u32,
    pub present_family: u32,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl VkBase {
    pub fn create(api_version: u32, app_name: &CStr, window: &Window) -> (Self, vk::SurfaceKHR) {
        let display_handle = window.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();

        let instance = Self::create_instance(display_handle, api_version, app_name);

        #[cfg(debug_assertions)]
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::Warning
                | vk::DebugUtilsMessageSeverityFlagsEXT::Error
                | vk::DebugUtilsMessageSeverityFlagsEXT::Verbose,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::General
                | vk::DebugUtilsMessageTypeFlagsEXT::Performance
                | vk::DebugUtilsMessageTypeFlagsEXT::Validation,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        };

        #[cfg(debug_assertions)]
        let utils_messenger = instance
            .create_debug_utils_messenger(&create_info, None)
            .unwrap();

        let surface = create_surface(&instance, display_handle, window_handle).unwrap();

        let (physical_device, graphics_family, present_family) =
            select_physical_device(&instance, surface);

        let device = Self::create_logical_device(
            &instance,
            physical_device,
            graphics_family,
            present_family,
        );

        let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family, 0) };

        (
            Self {
                instance,
                #[cfg(debug_assertions)]
                utils_messenger,
                physical_device,
                device,
                graphics_family,
                present_family,
                graphics_queue,
                present_queue,
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

        #[cfg(debug_assertions)]
        let layers: &[&CStr] = &[c"VK_LAYER_KHRONOS_validation"];
        #[cfg(not(debug_assertions))]
        let layers: &[&CStr] = &[];

        let supported_layers = vk::enumerate_instance_layer_properties().unwrap();

        // Layer filtering
        let active_layers: Vec<*const c_char> = layers
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
            enabled_layer_names: active_layers.as_ptr(),
            enabled_extension_names: extensions.as_ptr(),
            ..Default::default()
        };

        #[cfg(debug_assertions)]
        let mut debug = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::Error
                | vk::DebugUtilsMessageSeverityFlagsEXT::Warning
                | vk::DebugUtilsMessageSeverityFlagsEXT::Verbose,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::General
                | vk::DebugUtilsMessageTypeFlagsEXT::Performance
                | vk::DebugUtilsMessageTypeFlagsEXT::Validation,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        };

        #[cfg(debug_assertions)]
        let create_info = create_info.next(&mut debug);

        unsafe { vk::Instance::create(&create_info, None).unwrap() }
    }

    fn create_logical_device(
        instance: &vk::Instance,
        physical_device: vk::PhysicalDevice,
        graphics_family: u32,
        present_family: u32,
    ) -> vk::Device {
        let mut features11 = vk::PhysicalDeviceVulkan11Features {
            shader_draw_parameters: vk::TRUE,
            ..Default::default()
        };

        let extensions: &[&CStr] = &[khr::swapchain::NAME];

        let queue_create_infos: &[_] = if graphics_family == present_family {
            &[vk::DeviceQueueCreateInfo {
                queue_family_index: graphics_family,
                queue_priorities: &0.0,
                queue_count: 1,
                ..Default::default()
            }]
        } else {
            &[
                vk::DeviceQueueCreateInfo {
                    queue_family_index: graphics_family,
                    queue_priorities: &0.0,
                    queue_count: 1,
                    ..Default::default()
                },
                vk::DeviceQueueCreateInfo {
                    queue_family_index: present_family,
                    queue_priorities: &0.0,
                    queue_count: 1,
                    ..Default::default()
                },
            ]
        };

        let create_info = vk::DeviceCreateInfo {
            enabled_extension_names: extensions.as_ptr().cast(),
            enabled_extension_count: extensions.len() as u32,
            queue_create_info_count: queue_create_infos.len() as u32,
            queue_create_infos: queue_create_infos.as_ptr(),
            ..Default::default()
        }
        .next(&mut features11);

        unsafe {
            physical_device
                .create_device(&create_info, None, instance)
                .unwrap()
        }
    }

    pub fn destroy(&mut self) {
        self.device.destroy(None);
        #[cfg(debug_assertions)]
        self.instance
            .destroy_debug_utils_messenger(self.utils_messenger, None);
        self.instance.destroy(None)
    }
}

fn select_physical_device(
    instance: &vk::Instance,
    surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32, u32) {
    let devices = unsafe { instance.enumerate_physical_devices() }
        .expect("Bro how do you see this without a GPU?");

    let mut candidates: Vec<_> = devices
        .into_iter()
        .filter_map(|device| {
            let (gf, pf) = find_queue_families(device, surface)?;
            let props = device.get_properties();
            let score = match props.device_type {
                vk::PhysicalDeviceType::DiscreteGpu => 2,
                vk::PhysicalDeviceType::IntegratedGpu => 3,
                _ => 1,
            };
            Some((device, gf, pf, score))
        })
        .collect();

    candidates.sort_by_key(|c| std::cmp::Reverse(c.3));

    candidates
        .first()
        .map(|&(pd, gf, pf, _)| (pd, gf, pf))
        .expect("No suitable GPU!")
}

fn find_queue_families(
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Option<(u32, u32)> {
    let families = physical_device.get_queue_family_properties();

    let mut graphics = None;
    let mut present = None;

    for (i, family) in families.iter().enumerate() {
        if family.queue_flags.contains(vk::QueueFlags::Graphics) {
            graphics = Some(i as u32);
        }

        if physical_device
            .get_surface_support(i as u32, surface)
            .unwrap_or(false)
        {
            present = Some(i as u32);
        }

        if graphics.is_some() && present.is_some() {
            break;
        }
    }

    match (graphics, present) {
        (Some(g), Some(p)) => Some((g, p)),
        _ => None,
    }
}

#[cfg(debug_assertions)]
extern "system" fn debug_callback(
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
