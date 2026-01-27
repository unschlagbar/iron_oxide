use std::ffi::c_char;

use ash::{
    Entry, Instance,
    khr::{self, surface},
    prelude::VkResult,
    vk,
};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[cfg(target_os = "windows")]
use khr::win32_surface;

#[cfg(all(target_os = "linux", feature = "x11"))]
use khr::{xcb_surface, xlib_surface};

#[cfg(target_os = "linux")]
use khr::wayland_surface;

#[cfg(target_os = "android")]
use khr::android_surface;

#[cfg(any(target_os = "ios", target_os = "macos"))]
use ash::ext::metal_surface;

#[cfg(target_os = "macos")]
use raw_window_metal::{Layer, appkit};

#[cfg(target_os = "ios")]
use raw_window_metal::{Layer, uikit};

/// Create a surface from a raw display and window handle.
///
/// `instance` must have created with platform specific surface extensions enabled, acquired
/// through [`enumerate_required_extensions()`].
///
/// # Safety
///
/// There is a [parent/child relation] between [`Instance`] and [`Entry`], and the resulting
/// [`vk::SurfaceKHR`].  The application must not [destroy][Instance::destroy_instance()] these
/// parent objects before first [destroying][surface::Instance::destroy_surface()] the returned
/// [`vk::SurfaceKHR`] child object.  [`vk::SurfaceKHR`] does _not_ implement [drop][drop()]
/// semantics and can only be destroyed via [`destroy_surface()`][surface::Instance::destroy_surface()].
///
/// See the [`Entry::create_instance()`] documentation for more destruction ordering rules on
/// [`Instance`].
///
/// The window represented by `window_handle` must be associated with the display connection
/// in `display_handle`.
///
/// `window_handle` and `display_handle` must be associated with a valid window and display
/// connection, which must not be destroyed for the lifetime of the returned [`vk::SurfaceKHR`].
///
/// [parent/child relation]: https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#fundamentals-objectmodel-lifetime
pub fn create_surface(
    entry: &Entry,
    instance: &Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
) -> VkResult<vk::SurfaceKHR> {
    match (display_handle, window_handle) {
        #[cfg(target_os = "windows")]
        (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
            let surface_desc = vk::Win32SurfaceCreateInfoKHR {
                hwnd: window.hwnd.get(),
                hinstance: window
                    .hinstance
                    .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                    .get(),
                ..Default::default()
            };
            let surface_fn = win32_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_win32_surface(&surface_desc, None) }
        }

        #[cfg(target_os = "linux")]
        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
            let surface_desc = vk::WaylandSurfaceCreateInfoKHR {
                display: display.display.as_ptr(),
                surface: window.surface.as_ptr(),
                ..Default::default()
            };
            let surface_fn = wayland_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_wayland_surface(&surface_desc, None) }
        }

        #[cfg(all(target_os = "linux", feature = "x11"))]
        (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
            let surface_desc = vk::XlibSurfaceCreateInfoKHR {
                dpy: display
                    .display
                    .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                    .as_ptr(),
                window: window.window,
                ..Default::default()
            };

            let surface_fn = xlib_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_xlib_surface(&surface_desc, None) }
        }

        #[cfg(all(target_os = "linux", feature = "x11"))]
        (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
            let surface_desc = vk::XcbSurfaceCreateInfoKHR {
                connection: display
                    .connection
                    .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                    .as_ptr(),
                window: window.window.get(),
                ..Default::default()
            };
            let surface_fn = xcb_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_xcb_surface(&surface_desc, None) }
        }

        #[cfg(target_os = "android")]
        (RawDisplayHandle::Android(_), RawWindowHandle::AndroidNdk(window)) => {
            let surface_desc = vk::AndroidSurfaceCreateInfoKHR {
                window: window.a_native_window.as_ptr(),
                ..Default::default()
            };
            let surface_fn = android_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_android_surface(&surface_desc, None) }
        }

        #[cfg(target_os = "macos")]
        (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(window)) => {
            let layer = match appkit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
            let surface_fn = metal_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_metal_surface(&surface_desc, None) }
        }

        #[cfg(target_os = "ios")]
        (RawDisplayHandle::UiKit(_), RawWindowHandle::UiKit(window)) => {
            let layer = match uikit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
            let surface_fn = metal_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_metal_surface(&surface_desc, None) }
        }

        _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    }
}

/// Query the required instance extensions for creating a surface from a raw display handle.
///
/// This [`RawDisplayHandle`] can typically be acquired from a window, but is usually also
/// accessible earlier through an "event loop" concept to allow querying required instance
/// extensions and creation of a compatible Vulkan instance prior to creating a window.
///
/// The returned extensions will include all extension dependencies.
pub fn get_required_extensions(
    display_handle: RawDisplayHandle,
) -> VkResult<&'static [*const c_char]> {
    let extensions = match display_handle {
        #[cfg(target_os = "windows")]
        RawDisplayHandle::Windows(_) => {
            const WINDOWS_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];
            &WINDOWS_EXTS
        }

        #[cfg(target_os = "linux")]
        RawDisplayHandle::Wayland(_) => {
            const WAYLAND_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), wayland_surface::NAME.as_ptr()];
            &WAYLAND_EXTS
        }

        #[cfg(all(target_os = "linux", feature = "x11"))]
        RawDisplayHandle::Xlib(_) => {
            const XLIB_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), xlib_surface::NAME.as_ptr()];
            &XLIB_EXTS
        }

        #[cfg(all(target_os = "linux", feature = "x11"))]
        RawDisplayHandle::Xcb(_) => {
            const XCB_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), xcb_surface::NAME.as_ptr()];
            &XCB_EXTS
        }

        #[cfg(target_os = "android")]
        RawDisplayHandle::Android(_) => {
            const ANDROID_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), android_surface::NAME.as_ptr()];
            &ANDROID_EXTS
        }

        #[cfg(any(target_os = "ios", target_os = "macos"))]
        RawDisplayHandle::AppKit(_) | RawDisplayHandle::UiKit(_) => {
            const METAL_EXTS: [*const c_char; 2] =
                [surface::NAME.as_ptr(), metal_surface::NAME.as_ptr()];
            &METAL_EXTS
        }

        _ => return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    };

    Ok(extensions)
}
