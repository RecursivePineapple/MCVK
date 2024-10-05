use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::ffi::CStr;
use std::ptr::null;
use std::sync::Arc;

use ash::vk::AllocationCallbacks;
use ash::vk::SurfaceKHR;
use glfw::ffi::GLFWwindow;
use vulkano::device::physical::PhysicalDevice;
use vulkano::instance::Instance;
use vulkano::swapchain::Surface;
use vulkano::swapchain::SurfaceApi;
use vulkano::VulkanObject;

pub type GetRequiredInstanceExtensions =
    unsafe extern "C" fn(count: *mut c_uint) -> *const *const c_char;
pub type GetPhysicalDevicePresentationSupport = unsafe extern "C" fn(
    instance: <Instance as VulkanObject>::Handle,
    device: <PhysicalDevice as VulkanObject>::Handle,
    queue_index: c_uint,
) -> c_int;
pub type CreateWindowSurface = unsafe extern "C" fn(
    instance: <Instance as VulkanObject>::Handle,
    window: *mut GLFWwindow,
    allocator: *const AllocationCallbacks,
    surface: *mut SurfaceKHR,
) -> ash::vk::Result;
pub type GetWindowSize =
    unsafe extern "C" fn(window: *mut GLFWwindow, width: *mut c_int, height: *mut c_int);

#[derive(Debug, Clone, Copy)]
pub struct GLFWFns {
    pub get_required_instance_extensions: GetRequiredInstanceExtensions,
    pub get_physical_device_presentation_support: GetPhysicalDevicePresentationSupport,
    pub create_window_surface: CreateWindowSurface,
    pub get_window_size: GetWindowSize,
}

pub struct GLFWWindow {
    pub glfw: GLFWFns,
    pub window: *mut glfw::ffi::GLFWwindow,
}

unsafe impl Send for GLFWWindow {}
unsafe impl Sync for GLFWWindow {}

impl GLFWWindow {
    pub fn get_required_instance_extensions(&self) -> Vec<String> {
        unsafe {
            let mut count = 0;

            let exts = (self.glfw.get_required_instance_extensions)(&mut count);

            (0..count)
                .map(|i| *exts.add(i as usize).as_ref().unwrap())
                .map(|s| CStr::from_ptr(s).to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        }
    }

    pub fn get_window_size(&self) -> [u32; 2] {
        let mut image_extent = [0; 2];
        unsafe {
            let mut width = 0;
            let mut height = 0;
            (self.glfw.get_window_size)(self.window, &mut width, &mut height);
            image_extent[0] = width as u32;
            image_extent[1] = height as u32;
        }
        image_extent
    }

    pub fn create_surface(&self, instance: &Arc<Instance>) -> Arc<Surface> {
        let mut surface = Default::default();

        unsafe {
            (self.glfw.create_window_surface)(instance.handle(), self.window, null(), &mut surface)
                .result()
                .unwrap();
        }

        let surface = unsafe {
            Arc::new(Surface::from_handle(
                instance.clone(),
                surface,
                SurfaceApi::Xlib, // TODO: fix this
                None,
            ))
        };

        surface
    }

    pub fn get_physical_device_presentation_support(
        &self,
        instance: &Instance,
        device: &PhysicalDevice,
        queue: u32,
    ) -> bool {
        unsafe {
            (self.glfw.get_physical_device_presentation_support)(
                instance.handle(),
                device.handle(),
                queue,
            ) == 1
        }
    }
}
