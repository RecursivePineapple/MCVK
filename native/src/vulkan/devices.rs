use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::Device;
use vulkano::device::DeviceCreateInfo;
use vulkano::device::DeviceExtensions;
use vulkano::device::Queue;
use vulkano::device::QueueCreateInfo;
use vulkano::device::QueueFlags;
use vulkano::instance::Instance;
use vulkano::instance::InstanceCreateFlags;
use vulkano::instance::InstanceCreateInfo;
use vulkano::instance::InstanceExtensions;
use vulkano::Version;
use vulkano::VulkanLibrary;

use super::glfw_window::GLFWWindow;
use super::instance::VulkanInitError;
use super::utils::Ref;

pub struct Devices {
    pub instance: Arc<Instance>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl Devices {
    pub fn new(window: &Ref<GLFWWindow>) -> Result<Self, VulkanInitError> {
        let library = VulkanLibrary::new().unwrap();

        let mut inst_extensions = Vec::new();

        inst_extensions.append(&mut window.read().get_required_instance_extensions());

        let mut inst_extensions =
            InstanceExtensions::from_iter(inst_extensions.iter().map(|s| s.as_str()));
        inst_extensions.khr_surface = true;
        inst_extensions.khr_get_surface_capabilities2 = true;

        let mut inst_layers = Vec::new();

        if cfg!(debug_assertions) {
            let validation = library
                .layer_properties()
                .unwrap()
                .find(|l| l.name() == "VK_LAYER_KHRONOS_validation");

            inst_layers.push(
                validation
                    .expect("expected layer VK_LAYER_KHRONOS_validation to be present")
                    .name()
                    .to_owned(),
            );
        }

        info!(
            what = "creating vulkan instance",
            ?inst_extensions,
            ?inst_layers
        );
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: inst_extensions,
                enabled_layers: inst_layers,
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY, // required for MoltenVK on macOS
                max_api_version: Some(Version::V1_1),
                ..Default::default()
            },
        )?;

        let mut device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()?
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && window
                                .read()
                                .get_physical_device_presentation_support(&*instance, &*p, i as u32)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .ok_or(VulkanInitError::NoGPU)?;

        let pd_ext = physical_device.supported_extensions();
        let supports_excl_fullscreen = pd_ext.ext_full_screen_exclusive;
        device_extensions.ext_full_screen_exclusive = supports_excl_fullscreen;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        Ok(Self {
            instance,
            device,
            queue,
        })
    }
}
