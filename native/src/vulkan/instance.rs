use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, c_int, c_uint, CStr},
    ptr::null,
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use ash::vk;
use bytemuck::{Pod, Zeroable};
use enum_primitive::*;
use glfw::ffi::GLFWwindow;
use image::{GenericImage, GenericImageView, Rgba, RgbaImage};
use jni::objects::JObject;
use nalgebra_glm::{half_pi, look_at, perspective, TMat4};
use tracing::{info, warn};
use vulkano::{
    buffer::{Buffer, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferExecError, CommandBufferUsage, PrimaryCommandBufferAbstract,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageLayout, ImageUsage},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::{
        AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator,
    },
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::{Surface, SurfaceApi, Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    LoadingError, Validated, Version, VulkanError, VulkanLibrary, VulkanObject,
};

use crate::vulkan::{
    textures::textures::{TextureImage, TextureLoadError},
    utils::Extract,
};

use super::textures::textures::{self, AnimationMetadata, TextureHandle, TextureHandleData};

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

enum_from_primitive! {
    pub enum VsyncMode {
        Off = 0,
        On,
        Triple,
    }
}

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
    allocator: *const vk::AllocationCallbacks,
    surface: *mut vk::SurfaceKHR,
) -> vk::Result;
pub type GetWindowSize =
    unsafe extern "C" fn(window: *mut GLFWwindow, width: *mut c_int, height: *mut c_int);

#[derive(Debug, Clone, Copy)]
pub struct GLFWFns {
    pub get_required_instance_extensions: GetRequiredInstanceExtensions,
    pub get_physical_device_presentation_support: GetPhysicalDevicePresentationSupport,
    pub create_window_surface: CreateWindowSurface,
    pub get_window_size: GetWindowSize,
}

struct WindowSettings {
    pub vsync: VsyncMode,
    pub max_fps: Option<u32>,
}

struct SwapchainWrapper {
    pub render_pass: Arc<RenderPass>,
    pub allocator: Arc<StandardMemoryAllocator>,

    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,

    pub frame_buffers: Vec<Arc<Framebuffer>>,

    pub viewport: Viewport,
    pub projection: TMat4<f32>,
}

impl SwapchainWrapper {
    fn new(
        allocator: Arc<StandardMemoryAllocator>,
        render_pass: Arc<RenderPass>,
        swapchain: Arc<Swapchain>,
        images: Vec<Arc<Image>>,
    ) -> Self {
        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: (0.0..=1.0),
        };

        let mut projection = Default::default();

        let frame_buffers = Self::setup_swapchain(
            &allocator,
            &images,
            &render_pass,
            &mut viewport,
            &mut projection,
        );

        SwapchainWrapper {
            render_pass,
            allocator,
            swapchain,
            images,
            frame_buffers,
            viewport,
            projection,
        }
    }

    fn recreate(&mut self, new_dimensions: [u32; 2]) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: new_dimensions,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(Validated::Error(VulkanError::OutOfDate)) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        self.images = new_images;

        self.frame_buffers = Self::setup_swapchain(
            &self.allocator,
            &self.images,
            &self.render_pass,
            &mut self.viewport,
            &mut self.projection,
        );
    }

    fn setup_swapchain(
        allocator: &Arc<StandardMemoryAllocator>,
        images: &[Arc<Image>],
        render_pass: &Arc<RenderPass>,
        viewport: &mut Viewport,
        projection: &mut TMat4<f32>,
    ) -> Vec<Arc<Framebuffer>> {
        let extent = images[0].extent();
        viewport.extent = [extent[0] as f32, extent[1] as f32];

        let aspect_ratio = extent[0] as f32 / extent[1] as f32;
        *projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);

        let depth_buffer = ImageView::new_default(
            Image::new(
                allocator.clone(),
                ImageCreateInfo {
                    extent,
                    array_layers: 2,
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                    format: Format::D16_UNORM,
                    initial_layout: ImageLayout::Undefined,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        )
        .unwrap();

        let normal_buffer = ImageView::new_default(
            Image::new(
                allocator.clone(),
                ImageCreateInfo {
                    extent,
                    array_layers: 2,
                    usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                    format: Format::R16G16B16A16_SFLOAT,
                    initial_layout: ImageLayout::Undefined,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        )
        .unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view, normal_buffer.clone(), depth_buffer.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        framebuffers
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct VPData {
    pub view_projection: [[f32; 4]; 4],
}

pub struct VulkanInstance {
    instance: Arc<Instance>,

    glfw: GLFWFns,
    window: *mut glfw::ffi::GLFWwindow,
    surface: Arc<Surface>,

    device: Arc<Device>,
    queue: Arc<Queue>,
    queue_family_index: u32,

    memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,

    window_settings: WindowSettings,

    recreate_swapchain: bool,
    swapchain: SwapchainWrapper,

    vp_uniform: Subbuffer<VPData>,

    previous_frame_end: Option<Box<dyn GpuFuture>>,

    pending_textures: HashMap<String, Sprite>,
    textures: HashMap<String, TextureHandle>,
}

impl VulkanInstance {
    pub fn new(window: *mut glfw::ffi::GLFWwindow, glfw: GLFWFns) -> Result<Self, VulkanInitError> {
        let instance = {
            let library = VulkanLibrary::new().unwrap();

            let extensions = unsafe {
                let mut count = 0;

                let exts = (glfw.get_required_instance_extensions)(&mut count);

                (0..count)
                    .map(|i| *exts.add(i as usize).as_ref().unwrap())
                    .map(|s| CStr::from_ptr(s).to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            };

            let extensions = InstanceExtensions::from_iter(extensions.iter().map(|s| s.as_str()));

            let mut layers = Vec::new();

            if cfg!(debug_assertions) {
                let validation = library
                    .layer_properties()
                    .unwrap()
                    .find(|l| l.name() == "VK_LAYER_KHRONOS_validation");

                layers.push(
                    validation
                        .expect("expected layer VK_LAYER_KHRONOS_validation to be present")
                        .name()
                        .to_owned(),
                );
            }

            info!(what = "creating vulkan instance", ?extensions, ?layers);
            Instance::new(
                library,
                InstanceCreateInfo {
                    enabled_extensions: extensions,
                    enabled_layers: layers,
                    flags: InstanceCreateFlags::ENUMERATE_PORTABILITY, // required for MoltenVK on macOS
                    max_api_version: Some(Version::V1_1),
                    ..Default::default()
                },
            )?
        };

        let device_extensions = DeviceExtensions {
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
                        let valid_queue = unsafe {
                            1 == (glfw.get_physical_device_presentation_support)(
                                instance.handle(),
                                p.handle(),
                                i as u32,
                            )
                        };
                        dbg!(&valid_queue);

                        q.queue_flags.contains(QueueFlags::GRAPHICS) && valid_queue
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

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator =
            StandardDescriptorSetAllocator::new(device.clone(), Default::default());
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let mut surface = Default::default();

        unsafe {
            (glfw.create_window_surface)(instance.handle(), window, null(), &mut surface)
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

        let swapchain_image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        let (swapchain, images) = {
            let caps = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let usage = caps.supported_usage_flags;
            let alpha = caps.supported_composite_alpha.into_iter().next().unwrap();

            let mut image_extent = [0; 2];
            unsafe {
                let mut width = 0;
                let mut height = 0;
                (glfw.get_window_size)(window, &mut width, &mut height);
                image_extent[0] = width as u32;
                image_extent[1] = height as u32;
            }

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format: swapchain_image_format,
                    image_extent,
                    image_usage: usage,
                    composite_alpha: alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let render_pass = vulkano::ordered_passes_renderpass!(device.clone(),
            attachments: {
                color: {
                    format: swapchain_image_format,
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

        let window_settings = WindowSettings {
            vsync: VsyncMode::On,
            max_fps: None,
        };

        Ok(Self {
            instance,

            glfw,
            window,
            surface,

            device: device.clone(),
            queue,
            queue_family_index,

            memory_allocator: memory_allocator.clone(),
            descriptor_set_allocator,
            command_buffer_allocator,

            window_settings,

            recreate_swapchain: false,
            swapchain: SwapchainWrapper::new(
                memory_allocator.clone(),
                render_pass,
                swapchain,
                images,
            ),

            vp_uniform: Buffer::new_sized(
                memory_allocator.clone(),
                vulkano::buffer::BufferCreateInfo {
                    usage: BufferUsage::UNIFORM_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    ..Default::default()
                },
            )
            .unwrap(),

            previous_frame_end: Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>),

            pending_textures: HashMap::new(),
            textures: HashMap::new(),
        })
    }
}

impl VulkanInstance {
    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        // TODO: fix this
    }

    pub fn set_vsync(&mut self, vsync: VsyncMode) {
        // TODO: fix this
    }
}

impl VulkanInstance {
    pub fn start_frame(&mut self, mc: JObject<'_>) {
        self.render(&mc);
    }

    pub fn finish_frame(&mut self) {
        todo!()
    }

    fn render(&mut self, mc: &JObject<'_>) {
        self.previous_frame_end
            .as_mut()
            .take()
            .unwrap()
            .cleanup_finished();

        if self.recreate_swapchain {
            let mut image_extent = [0; 2];
            unsafe {
                let mut width = 0;
                let mut height = 0;
                (self.glfw.get_window_size)(self.window, &mut width, &mut height);
                image_extent[0] = width as u32;
                image_extent[1] = height as u32;
            }

            self.swapchain.recreate(image_extent);

            self.recreate_swapchain = false;
        }

        self.update_uniforms(mc);

        todo!();
    }

    fn update_uniforms(&mut self, mc: &JObject<'_>) {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TextureMapLoadError {
    #[error("could not create command buffer builder: {0}")]
    CommandBufferCreateError(Validated<VulkanError>),
    #[error("could not build command buffer: {0}")]
    CommandBufferBuildError(Validated<VulkanError>),
    #[error("could not execute command buffer: {0}")]
    CommandBufferExecError(#[from] CommandBufferExecError),
    #[error("could not signal_fence_and_flush: {0}")]
    SignalFenceFlushError(Validated<VulkanError>),
    #[error("could not wait for texture load to finish: {0}")]
    WaitError(Validated<VulkanError>),
    #[error("could not load the no-texture texture: {0}")]
    NoMissingNo(#[from] TextureLoadError),
}

pub enum Sprite {
    Missing,
    Data {
        data: Vec<u8>,
        animation: Option<AnimationMetadata>,
    },
    Image {
        image: RgbaImage,
        animation: Option<AnimationMetadata>,
    },
    Frames {
        frames: Vec<RgbaImage>,
        animation: Option<AnimationMetadata>,
    },
}

const MISSINGNO: &'static str = "missingno";

fn get_missingno() -> RgbaImage {
    let black = Rgba(0x00_00_00_FF_u32.to_ne_bytes());
    let pink = Rgba(0xF8_00_F8_FF_u32.to_ne_bytes());

    let mut image = RgbaImage::new(16, 16);

    for y in 0..16 {
        for x in 0..16 {
            let color = (y >= 8) ^ (x >= 8);

            image.put_pixel(x, y, if color { pink } else { black });
        }
    }

    image
}

impl VulkanInstance {
    pub fn enqueue_sprite(&mut self, name: String, sprite: Sprite) {
        self.pending_textures.insert(name, sprite);
    }

    pub fn load_sprites(
        &mut self,
        max_mipmap_levels: u32,
        gen_aniso_data: bool,
    ) -> Result<(), TextureMapLoadError> {
        let pending_textures = std::mem::take(&mut self.pending_textures);

        info!(what = "loading textures", count = pending_textures.len());
        let start = Instant::now();
        let total_start = start.clone();

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(TextureMapLoadError::CommandBufferCreateError)?;

        let missingno = textures::load_from_image(
            self.memory_allocator.clone(),
            &mut commands,
            TextureImage::Static {
                image: get_missingno(),
            },
            max_mipmap_levels,
            gen_aniso_data,
            None,
        )?;

        let missingno = Arc::new(missingno);

        let textures = pending_textures
            .into_iter()
            .map(|(name, sprite)| {
                #[rustfmt::skip]
                let texture = match sprite {
                    Sprite::Missing => {
                        Ok(missingno.clone())
                    },
                    Sprite::Data { data, animation } => {
                        textures::load_from_data(
                            self.memory_allocator.clone(),
                            &mut commands,
                            &data[..],
                            max_mipmap_levels,
                            gen_aniso_data,
                            animation,
                        ).map(Arc::new)
                    },
                    Sprite::Image { image, animation } => {
                        TextureImage::from_spritesheet(image)
                            .and_then(|sprites| {
                                textures::load_from_image(
                                    self.memory_allocator.clone(),
                                    &mut commands,
                                    sprites,
                                    max_mipmap_levels,
                                    gen_aniso_data,
                                    animation,
                                )
                            })
                            .map(Arc::new)
                    },
                    Sprite::Frames { frames, animation } => {
                        textures::load_from_image(
                            self.memory_allocator.clone(),
                            &mut commands,
                            TextureImage::Frames {
                                width: frames[0].width(),
                                height: frames[0].height(),
                                frames,
                            },
                            max_mipmap_levels,
                            gen_aniso_data,
                            animation,
                        ).map(Arc::new)
                    },
                };

                match texture {
                    Ok(t) => (name, t),
                    Err(err) => {
                        warn!(
                            what = "could not load texture: it will be replaced with missingno",
                            who = name,
                            why = %err
                        );

                        (name, missingno.clone())
                    }
                }
            })
            .chain(vec![(MISSINGNO.to_owned(), missingno.clone())].into_iter())
            .collect::<Vec<_>>();

        info!(
            what = "finished loading texture images",
            elapsed_secs = (Instant::now() - start).as_secs_f32()
        );
        let start = Instant::now();

        let command_buffer = commands
            .build()
            .map_err(TextureMapLoadError::CommandBufferBuildError)?;

        let fut = self
            .previous_frame_end
            .take()
            .unwrap()
            .then_execute(self.queue.clone(), command_buffer)
            .map_err(TextureMapLoadError::CommandBufferExecError)?
            .then_signal_fence_and_flush()
            .map_err(TextureMapLoadError::SignalFenceFlushError)?;

        self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));

        let mut old_textures = self.textures.extract();
        self.textures.reserve(old_textures.len());

        for (name, texture) in textures {
            match old_textures.remove(&name) {
                Some(existing) => {
                    existing.replace(texture);
                    self.textures.insert(name, existing);
                }
                None => {
                    self.textures.insert(name, TextureHandle::new(texture));
                }
            }
        }

        for (name, old_texture) in old_textures {
            warn!(
                what = "a texture does not exist in this resource pack",
                name
            );
            old_texture.replace(Arc::new(textures::Texture::None));
        }

        info!(
            what = "finished updating the texture map",
            elapsed_secs = (Instant::now() - start).as_secs_f32()
        );
        let start = Instant::now();

        fut.wait(None).map_err(TextureMapLoadError::WaitError)?;

        info!(
            what = "finished loading textures",
            elapsed_secs = (Instant::now() - start).as_secs_f32(),
            elapsed_total_secs = (Instant::now() - total_start).as_secs_f32(),
        );

        Ok(())
    }
}
