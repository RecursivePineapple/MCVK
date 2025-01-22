use enum_primitive::*;
use nalgebra_glm::half_pi;
use nalgebra_glm::perspective;
use nalgebra_glm::TMat4;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::view::ImageViewCreateInfo;
use vulkano::image::view::ImageViewType;
use vulkano::image::Image;
use vulkano::image::ImageCreateInfo;
use vulkano::image::ImageLayout;
use vulkano::image::ImageUsage;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::Framebuffer;
use vulkano::render_pass::FramebufferCreateInfo;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::acquire_next_image;
use vulkano::swapchain::FullScreenExclusive;
use vulkano::swapchain::PresentGravity;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::PresentScaling;
use vulkano::swapchain::Surface;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::SwapchainAcquireFuture;
use vulkano::swapchain::SwapchainCreateInfo;
use vulkano::Validated;
use vulkano::VulkanError;

use super::devices::Devices;
use super::glfw_window::GLFWWindow;
use super::instance::Allocators;
use super::utils::Ref;

enum_from_primitive! {
    pub enum VsyncMode {
        Off = 0,
        On,
        Triple,
    }
}

pub struct WindowSettings {
    pub vsync: VsyncMode,
    pub max_fps: Option<u32>,
}

pub struct SwapchainManager {
    window: Ref<GLFWWindow>,
    devices: Ref<Devices>,
    allocator: Ref<Allocators>,

    pub window_settings: WindowSettings,

    pub surface: Option<Arc<Surface>>,

    pub render_pass: Option<Arc<RenderPass>>,

    pub image_format: Option<Format>,
    pub swapchain: Option<Arc<Swapchain>>,
    pub images: Option<Vec<Arc<Image>>>,
    pub recreate_swapchain: bool,
    pub acquired_image: Option<i32>,

    pub frame_buffers: Option<Vec<Arc<Framebuffer>>>,

    pub viewport: Viewport,
    pub projection: TMat4<f32>,
}

const SWAPCHAIN_IMAGE_COUNT: u32 = 4;

impl SwapchainManager {
    pub fn new(window: Ref<GLFWWindow>, devices: Ref<Devices>, allocator: Ref<Allocators>) -> Self {
        let mut this = SwapchainManager {
            window,
            devices,
            allocator,
            window_settings: WindowSettings {
                vsync: VsyncMode::On,
                max_fps: None,
            },
            surface: None,
            render_pass: None,
            image_format: None,
            swapchain: None,
            images: None,
            recreate_swapchain: false,
            acquired_image: None,
            frame_buffers: None,
            viewport: Viewport::default(),
            projection: TMat4::identity(),
        };

        this.create_swapchain();

        this
    }

    pub fn create_swapchain(&mut self) {
        if self.surface.is_none() {
            self.surface = Some(
                self.window
                    .read()
                    .create_surface(&self.devices.read().instance),
            );

            self.image_format = Some(
                self.devices
                    .read()
                    .device
                    .physical_device()
                    .surface_formats(self.surface.as_ref().unwrap(), Default::default())
                    .unwrap()[0]
                    .0,
            );

            self.swapchain = None;
        }

        if let Some(current) = self.swapchain.clone() {
            let (new_swapchain, new_images) = match current.recreate(SwapchainCreateInfo {
                image_extent: self.window.read().get_window_size(),
                image_format: self.image_format.clone().unwrap(),
                present_mode: match self.window_settings.vsync {
                    VsyncMode::Off => PresentMode::Immediate,
                    VsyncMode::On => PresentMode::FifoRelaxed,
                    VsyncMode::Triple => PresentMode::Mailbox,
                },
                ..current.create_info()
            }) {
                Ok(r) => r,
                Err(Validated::Error(VulkanError::OutOfDate)) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = Some(new_swapchain);
            self.images = Some(new_images);
        } else {
            let caps = self
                .devices
                .read()
                .device
                .physical_device()
                .surface_capabilities(self.surface.as_ref().unwrap(), Default::default())
                .unwrap();

            let usage = caps.supported_usage_flags;
            let alpha = caps.supported_composite_alpha.into_iter().next().unwrap();

            let (swapchain, images) = Swapchain::new(
                self.devices.read().device.clone(),
                self.surface.clone().unwrap(),
                SwapchainCreateInfo {
                    min_image_count: SWAPCHAIN_IMAGE_COUNT,
                    image_format: self.image_format.clone().unwrap(),
                    image_extent: self.window.read().get_window_size(),
                    image_usage: usage,
                    composite_alpha: alpha,
                    present_mode: match self.window_settings.vsync {
                        VsyncMode::Off => PresentMode::Immediate,
                        VsyncMode::On => PresentMode::FifoRelaxed,
                        VsyncMode::Triple => PresentMode::Mailbox,
                    },
                    scaling_behavior: Some(PresentScaling::AspectRatioStretch),
                    present_gravity: Some(
                        [PresentGravity::Centered, PresentGravity::Centered].into(),
                    ),
                    full_screen_exclusive: if self
                        .devices
                        .read()
                        .device
                        .enabled_extensions()
                        .ext_full_screen_exclusive
                    {
                        FullScreenExclusive::Allowed
                    } else {
                        FullScreenExclusive::Default
                    },
                    ..Default::default()
                },
            )
            .unwrap();

            self.swapchain = Some(swapchain);
            self.images = Some(images);
        }

        self.update_viewport();
        self.create_framebuffers();

        self.recreate_swapchain = false;
    }

    pub fn update_viewport(&mut self) {
        let extent = self.images.as_ref().unwrap()[0].extent();
        self.viewport.extent = [extent[0] as f32, extent[1] as f32];

        let aspect_ratio = extent[0] as f32 / extent[1] as f32;
        self.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);
    }

    pub fn create_framebuffers(&mut self) {
        self.frame_buffers = None;

        if let Some(render_pass) = self.render_pass.as_ref() {
            let extent = self.images.as_ref().unwrap()[0].extent();

            let depth_buffer = Image::new(
                self.allocator.read().memory_allocator.clone(),
                ImageCreateInfo {
                    extent,
                    array_layers: self.images.as_ref().unwrap().len() as u32,
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                    format: Format::D16_UNORM,
                    initial_layout: ImageLayout::Undefined,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap();

            let normal_buffer = Image::new(
                self.allocator.read().memory_allocator.clone(),
                ImageCreateInfo {
                    extent,
                    array_layers: self.images.as_ref().unwrap().len() as u32,
                    usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                    format: Format::R16G16B16A16_SFLOAT,
                    initial_layout: ImageLayout::Undefined,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap();

            self.frame_buffers = Some(
                self.images
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, image)| {
                        let i = i as u32;

                        let view = ImageView::new_default(image.clone()).unwrap();

                        let mut normals_range = normal_buffer.subresource_range();
                        normals_range.array_layers = i..(i + 1);

                        let mut depth_range = normal_buffer.subresource_range();
                        depth_range.array_layers = i..(i + 1);

                        Framebuffer::new(
                            render_pass.clone(),
                            FramebufferCreateInfo {
                                attachments: vec![
                                    view,
                                    ImageView::new(
                                        normal_buffer.clone(),
                                        ImageViewCreateInfo {
                                            view_type: ImageViewType::Dim2d,
                                            format: normal_buffer.format(),
                                            subresource_range: normals_range,
                                            ..Default::default()
                                        },
                                    )
                                    .unwrap(),
                                    ImageView::new(
                                        depth_buffer.clone(),
                                        ImageViewCreateInfo {
                                            view_type: ImageViewType::Dim2d,
                                            format: normal_buffer.format(),
                                            subresource_range: depth_range,
                                            ..Default::default()
                                        },
                                    )
                                    .unwrap(),
                                ],
                                ..Default::default()
                            },
                        )
                        .unwrap()
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    pub fn acquire_image(&mut self) -> (u32, SwapchainAcquireFuture) {
        debug!(what = "acquiring next swapchain image");

        let image_index;
        let acquire_future;

        loop {
            let x = match acquire_next_image(
                self.swapchain.clone().unwrap(),
                Some(Duration::from_millis(1000)),
            ) {
                Ok(r) => r,
                Err(Validated::Error(VulkanError::OutOfDate))
                | Err(Validated::Error(VulkanError::FullScreenExclusiveModeLost)) => {
                    debug!(what = "recreating the swapchain within acquire_image");
                    self.create_swapchain();
                    continue;
                }
                Err(Validated::Error(VulkanError::SurfaceLost)) => {
                    self.surface = None;
                    debug!(
                        what = "recreating the swapchain within acquire_image due to lost surface"
                    );
                    self.create_swapchain();
                    continue;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

            let suboptimal;
            (image_index, suboptimal, acquire_future) = x;

            if suboptimal {
                debug!(what = "swapchain image was suboptimal");
                self.recreate_swapchain = true;
            }

            break;
        }

        debug!(what = "acquired swapchain image", image_index);
        (image_index, acquire_future)
    }
}
