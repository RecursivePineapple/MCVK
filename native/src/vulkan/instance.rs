use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use enum_primitive::*;
use tracing::info;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBufferUsage;
use vulkano::command_buffer::PrimaryCommandBufferAbstract;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::format::Format;
use vulkano::image::ImageLayout;
use vulkano::memory::allocator::FreeListAllocator;
use vulkano::memory::allocator::GenericMemoryAllocator;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::sync::GpuFuture;
use vulkano::LoadingError;
use vulkano::Validated;
use vulkano::VulkanError;

use crate::vulkan::textures::textures::TextureImage;

use super::devices::Devices;
use super::glfw_window::GLFWWindow;
use super::render_manager::RenderManager;
use super::spinlock::SpinLock;
use super::swapchain::SwapchainManager;
use super::swapchain::VsyncMode;
use super::textures::texture_manager::TextureHandle;
use super::textures::texture_manager::TextureParams;
use super::textures::texture_manager::TextureReference;
use super::textures::texture_manager::TextureStorage;
use super::utils::Ref;

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

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct TextureManager {
    allocators: Ref<Allocators>,
    rendering: Ref<RenderManager>,

    pub texture_storage: TextureStorage,

    pub is_resource_pack_reload: bool,
    pub unupdated_textures: HashSet<String>,

    pub textures_by_id: HashMap<u32, Arc<TextureHandle>>,
    pub textures_by_name: HashMap<String, Arc<TextureHandle>>,
    pub next_texture_id: u32,
}

impl TextureManager {
    pub fn new(allocators: &Ref<Allocators>, rendering: &Ref<RenderManager>) -> Self {
        Self {
            allocators: allocators.clone(),
            rendering: rendering.clone(),

            texture_storage: TextureStorage::new(allocators),

            is_resource_pack_reload: false,
            unupdated_textures: HashSet::new(),

            textures_by_id: HashMap::new(),
            textures_by_name: HashMap::new(),
            next_texture_id: 0,
        }
    }

    pub fn begin_texture_reload(&mut self) {
        self.is_resource_pack_reload = true;
        self.unupdated_textures = self.textures_by_name.keys().cloned().collect();
    }

    pub fn create_texture(&mut self) -> u32 {
        let id = self.next_texture_id;
        self.textures_by_id.insert(
            id,
            Arc::new(TextureHandle {
                resource_name: None,
                texture_id: id,
                texture: SpinLock::new(Arc::new(TextureReference::None)),
                animation: None,
                mipmapped: false,
                params: SpinLock::new(TextureParams::default()),
            }),
        );

        self.next_texture_id += 1;

        id
    }

    pub fn free_texture(&mut self, id: u32) {
        if let Some(t) = self.textures_by_id.remove(&id) {
            if let Some(name) = t.resource_name.as_ref() {
                self.textures_by_name.remove(name);
            }
        }
    }

    pub fn get_texture_handle(&self, id: u32) -> Option<&Arc<TextureHandle>> {
        self.textures_by_id.get(&id)
    }

    pub fn enqueue_sprite(
        &mut self,
        name: String,
        image: TextureImage,
    ) -> Result<Arc<TextureHandle>, anyhow::Error> {
        self.unupdated_textures.remove(&name);

        let handle = match self.textures_by_name.get(&name) {
            Some(handle) => handle.clone(),
            None => {
                let handle = Arc::new(TextureHandle {
                    resource_name: Some(name.clone()),
                    texture_id: self.next_texture_id,
                    texture: SpinLock::new(Arc::new(TextureReference::None)),
                    animation: None,
                    mipmapped: true,
                    params: SpinLock::new(TextureParams::default()),
                });

                self.next_texture_id += 1;

                self.textures_by_name.insert(name.clone(), handle.clone());
                self.textures_by_id
                    .insert(handle.texture_id, handle.clone());

                handle
            }
        };

        let image = image
            .load()
            .with_context(|| format!("could not load image data for texture {name}"))?;

        if matches!(image, TextureImage::None) {
            handle
                .texture
                .set(self.texture_storage.get_missingno().clone());

            return Ok(handle);
        }

        self.texture_storage
            .enqueue_handle_update(&handle, image)
            .with_context(|| format!("could not update gpu texture for texture {name}"))?;

        Ok(handle)
    }

    pub fn finish_texture_reload(&mut self) -> anyhow::Result<()> {
        for skipped in self.unupdated_textures.drain() {
            tracing::warn!(
                what = "texture has been skipped in resource reload",
                who = skipped
            );

            let handle = self.textures_by_name.remove(&skipped).unwrap();
            self.textures_by_id.remove(&handle.texture_id);

            // free the backing texture
            handle
                .texture
                .set(self.texture_storage.get_missingno().clone());
        }

        info!(what = "waiting for all frames to finish for texture reload");

        let mut renderer = self.rendering.borrow_mut();

        renderer.flush()?;

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.allocators.borrow().command_buffer_allocator,
            renderer.queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let pre_record = Instant::now();

        self.texture_storage.record_commands(&mut commands);

        let commands = commands.build()?;

        let post_record = Instant::now();

        let fut = commands
            .execute(renderer.queue().clone())?
            .boxed()
            .then_signal_fence_and_flush()?;

        fut.wait(None).unwrap();

        let post_upload = Instant::now();

        info!(
            what = "uploaded all gpu textures",
            count = self.textures_by_name.len(),
            record_duration_secs = (post_record - pre_record).as_secs_f32(),
            upload_duration_secs = (post_upload - post_record).as_secs_f32()
        );

        Ok(())
    }
}

pub struct Allocators {
    pub memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    pub descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
}

impl Allocators {
    pub fn new(devices: &Ref<Devices>) -> Self {
        Self {
            memory_allocator: Arc::new(StandardMemoryAllocator::new_default(
                devices.borrow().device.clone(),
            )),
            descriptor_set_allocator: StandardDescriptorSetAllocator::new(
                devices.borrow().device.clone(),
                Default::default(),
            ),
            command_buffer_allocator: StandardCommandBufferAllocator::new(
                devices.borrow().device.clone(),
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
        let devices = Arc::new(RefCell::new(Devices::new(&window)?));

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
            devices.borrow().device.clone(),
        ));
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(
            devices.borrow().device.clone(),
            Default::default(),
        );
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            devices.borrow().device.clone(),
            Default::default(),
        );

        let allocators = Arc::new(RefCell::new(Allocators {
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
        }));

        let swapchain = Arc::new(RefCell::new(SwapchainManager::new(
            window.clone(),
            devices.clone(),
            allocators.clone(),
        )));

        let render_pass = vulkano::ordered_passes_renderpass!(devices.borrow().device.clone(),
            attachments: {
                color: {
                    format: swapchain.borrow().image_format.clone().unwrap(),
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
                    // sky
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    // world (solid)
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    // world (transparent)
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    // ui
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                }
            ]
        )
        .unwrap();

        swapchain.borrow_mut().render_pass = Some(render_pass.clone());
        swapchain.borrow_mut().create_framebuffers();

        let rendering = Arc::new(RefCell::new(RenderManager::new(
            &allocators,
            &devices,
            &swapchain,
        )));

        let textures = Arc::new(RefCell::new(TextureManager::new(&allocators, &rendering)));

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
        self.swapchain.borrow_mut().window_settings.max_fps = max_fps;
    }

    pub fn set_vsync(&mut self, vsync: VsyncMode) {
        self.swapchain.borrow_mut().window_settings.vsync = vsync;
        self.swapchain.borrow_mut().recreate_swapchain = true;
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
