use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use bytemuck::Pod;
use bytemuck::Zeroable;
use enum_primitive::*;
use nalgebra::Matrix4;
use nalgebra_glm::half_pi;
use nalgebra_glm::perspective;
use nalgebra_glm::TMat4;
use tracing::debug;
use tracing::info;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBufferUsage;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::PrimaryCommandBufferAbstract;
use vulkano::command_buffer::RenderPassBeginInfo;
use vulkano::command_buffer::SubpassBeginInfo;
use vulkano::command_buffer::SubpassContents;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::Device;
use vulkano::device::DeviceCreateInfo;
use vulkano::device::DeviceExtensions;
use vulkano::device::Queue;
use vulkano::device::QueueCreateInfo;
use vulkano::device::QueueFlags;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::view::ImageViewCreateInfo;
use vulkano::image::view::ImageViewType;
use vulkano::image::Image;
use vulkano::image::ImageCreateInfo;
use vulkano::image::ImageLayout;
use vulkano::image::ImageUsage;
use vulkano::instance::Instance;
use vulkano::instance::InstanceCreateFlags;
use vulkano::instance::InstanceCreateInfo;
use vulkano::instance::InstanceExtensions;
use vulkano::memory::allocator::FreeListAllocator;
use vulkano::memory::allocator::GenericMemoryAllocator;
use vulkano::memory::allocator::StandardMemoryAllocator;
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
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;
use vulkano::LoadingError;
use vulkano::Validated;
use vulkano::Version;
use vulkano::VulkanError;
use vulkano::VulkanLibrary;

use crate::vulkan::textures::textures::TextureImage;

use super::devices::Devices;
use super::glfw_window::GLFWWindow;
use super::instance::Allocators;
use super::shaders::uniforms::Uniform;
use super::spinlock::SpinLock;
use super::swapchain::SwapchainManager;
use super::textures::texture_manager::TextureHandle;
use super::textures::texture_manager::TextureParams;
use super::textures::texture_manager::TextureReference;
use super::textures::texture_manager::TextureStorage;
use super::utils::Ref;

struct Frame {
    pub future: FenceSignalFuture<Box<dyn GpuFuture>>,
    pub resources: LinkedList<Arc<dyn Drop>>,
}

pub struct RenderManager {
    swapchain: Ref<SwapchainManager>,
    allocators: Ref<Allocators>,

    queue: Arc<Queue>,

    frames_in_flight: HashMap<u32, Frame>,
    frame_counter: u32,

    view: Matrix4<f32>,
    vp: Uniform<TMat4<f32>>,

    command_buffer: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    swapchain_index: Option<u32>,
    swapchain_future: Option<SwapchainAcquireFuture>,
}

impl RenderManager {
    pub fn new(
        allocators: &Ref<Allocators>,
        device: &Ref<Devices>,
        swapchain: &Ref<SwapchainManager>,
    ) -> Self {
        Self {
            swapchain: swapchain.clone(),
            allocators: allocators.clone(),

            queue: device.borrow().queue.clone(),

            frames_in_flight: HashMap::new(),
            frame_counter: 0,

            view: TMat4::identity(),
            vp: Uniform::new(allocators, TMat4::identity()).unwrap(),

            command_buffer: None,
            swapchain_index: None,
            swapchain_future: None,
        }
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    pub fn flush(&mut self) -> Result<(), Validated<VulkanError>> {
        for (_, frame) in self.frames_in_flight.drain() {
            frame.future.wait(None)?;
        }
        Ok(())
    }

    pub fn end_frame(&mut self) {
        self.frame_counter += 1;
    }

    pub fn start_frame(&mut self) {
        let mut swapchain = self.swapchain.borrow_mut();

        if swapchain.recreate_swapchain {
            swapchain.create_swapchain();
        }

        self.vp.data = self.view * swapchain.projection;
        self.vp.upload().unwrap();

        let (swapchain_index, swapchain_future) = swapchain.acquire_image();
        self.swapchain_index = Some(swapchain_index);
        self.swapchain_future = Some(swapchain_future);

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.allocators.borrow().command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        commands
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some([0.0, 0.0, 0.0, 1.0].into()),
                        Some([0.0, 0.0, 0.0, 1.0].into()),
                        Some([0.0, 0.0, 0.0, 1.0].into()),
                        Some(1.0.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(
                        swapchain.frame_buffers.as_ref().unwrap()[swapchain_index as usize].clone(),
                    )
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap()
            .set_viewport(0, vec![swapchain.viewport.clone()].into());

        self.command_buffer = Some(commands);
    }
}
