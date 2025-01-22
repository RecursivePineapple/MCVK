use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use enum_primitive::*;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::format::Format;
use vulkano::image::ImageLayout;
use vulkano::memory::allocator::FreeListAllocator;
use vulkano::memory::allocator::GenericMemoryAllocator;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::LoadingError;
use vulkano::Validated;
use vulkano::VulkanError;

use super::devices::Devices;
use super::glfw_window::GLFWWindow;
use super::render_manager::RenderManager;
use super::swapchain::SwapchainManager;
use super::swapchain::VsyncMode;
use super::textures::texture_manager::TextureManager;
use super::utils::Ref;

pub static MAIN_THREAD: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error)]
pub enum VulkanInitError {
    #[error("system does not support vulkan: {0}")]
    NoVulkan(#[from] LoadingError),
    #[error("could not create vulkan instance: {0}")]
    BadInstanceParams(#[from] Validated<VulkanError>),
    #[error("{0}")]
    VulkanError(#[from] VulkanError),
    #[error("there are no GPUs present which support vulkan")]
    NoGPU,
}

#[derive(Debug)]
pub struct Allocators {
    pub memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    pub descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
}

impl Allocators {
    pub fn new(devices: &Ref<Devices>) -> Self {
        Self {
            memory_allocator: Arc::new(StandardMemoryAllocator::new_default(
                devices.read().device.clone(),
            )),
            descriptor_set_allocator: StandardDescriptorSetAllocator::new(
                devices.read().device.clone(),
                Default::default(),
            ),
            command_buffer_allocator: StandardCommandBufferAllocator::new(
                devices.read().device.clone(),
                Default::default(),
            ),
        }
    }
}

pub struct MCVK {
    pub window: Ref<GLFWWindow>,
    pub allocators: Ref<Allocators>,
    pub devices: Ref<Devices>,
    pub swapchain: Ref<SwapchainManager>,
    pub textures: Ref<TextureManager>,
    pub rendering: Ref<RenderManager>,
}

unsafe impl Send for MCVK {}
unsafe impl Sync for MCVK {}

impl MCVK {
    pub fn new(window: Ref<GLFWWindow>) -> Result<Self, VulkanInitError> {
        MAIN_THREAD.store(
            std::thread::current().id().as_u64().into(),
            Ordering::Release,
        );

        let devices = Ref::new(Devices::new(&window)?);

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
            devices.read().device.clone(),
        ));
        let descriptor_set_allocator =
            StandardDescriptorSetAllocator::new(devices.read().device.clone(), Default::default());
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(devices.read().device.clone(), Default::default());

        let allocators = Ref::new(Allocators {
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
        });

        let swapchain = Ref::new(SwapchainManager::new(
            window.clone(),
            devices.clone(),
            allocators.clone(),
        ));

        let render_pass = vulkano::ordered_passes_renderpass!(devices.read().device.clone(),
            attachments: {
                color: {
                    format: swapchain.read().image_format.clone().unwrap(),
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                    initial_layout: ImageLayout::Preinitialized,
                    final_layout: ImageLayout::ColorAttachmentOptimal
                },
                normals: {
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                    initial_layout: ImageLayout::Undefined,
                    final_layout: ImageLayout::ColorAttachmentOptimal
                },
                depth: {
                    format: Format::D16_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                    initial_layout: ImageLayout::Undefined,
                    final_layout: ImageLayout::DepthStencilAttachmentOptimal
                }
            },
            passes: [
                {
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                }
            ]
        )
        .unwrap();

        swapchain.write().render_pass = Some(render_pass.clone());
        swapchain.write().create_framebuffers();

        let rendering = Ref::new(RenderManager::new(&allocators, &devices, &swapchain));

        let textures = Ref::new(TextureManager::new(&allocators, &rendering));

        Ok(Self {
            window,
            devices,
            allocators,
            swapchain,
            textures,
            rendering,
        })
    }
}

impl MCVK {
    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        self.swapchain.write().window_settings.max_fps = max_fps;
    }

    pub fn set_vsync(&mut self, vsync: VsyncMode) {
        self.swapchain.write().window_settings.vsync = vsync;
        self.swapchain.write().recreate_swapchain = true;
    }
}

impl MCVK {
    /// Colour clears are the boundary marker between frames. We use this to sync pretty much everything.
    pub fn on_clear_colour(&mut self) {
        // by this point all possible render insns have been generated, stored, and ideally transformed into render commands
        // this function should submit the render commands and do all swapchain swapping
        // for now we will make this call blocking but it must be non-blocking for good performance (record all insns and generate the commands -vsync> submit & draw)
    }
}
