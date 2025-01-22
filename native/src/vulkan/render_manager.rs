use std::collections::HashMap;
use std::collections::LinkedList;
use std::sync::Arc;

use anyhow::Result;
use enum_primitive::*;
use nalgebra::Matrix4;
use nalgebra_glm::TMat4;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBufferUsage;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::RenderPassBeginInfo;
use vulkano::command_buffer::SubpassBeginInfo;
use vulkano::command_buffer::SubpassContents;
use vulkano::device::Queue;
use vulkano::swapchain::SwapchainAcquireFuture;
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;
use vulkano::Validated;
use vulkano::VulkanError;

use super::devices::Devices;
use super::instance::Allocators;
use super::shaders::uniforms::Uniform;
use super::swapchain::SwapchainManager;
use super::utils::MainRenderThread;
use super::utils::Ref;

/// A reference to a resource to keep it from being dropped prematurely.
pub type ResourceReference = Arc<dyn Drop + Send + Sync + 'static>;

struct Frame {
    pub future: MainRenderThread<FenceSignalFuture<Box<dyn GpuFuture>>>,
    pub resources: LinkedList<ResourceReference>,
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct RenderManager {
    swapchain: Ref<SwapchainManager>,
    allocators: Ref<Allocators>,

    queue: Arc<Queue>,

    frames_in_flight: HashMap<u32, Frame>,
    frame_counter: u32,

    view: Matrix4<f32>,
    vp: Uniform<TMat4<f32>>,

    command_buffer: Option<MainRenderThread<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>>,
    swapchain_index: Option<u32>,
    swapchain_future: Option<MainRenderThread<SwapchainAcquireFuture>>,

    used_resources: LinkedList<ResourceReference>,
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

            queue: device.read().queue.clone(),

            frames_in_flight: HashMap::new(),
            frame_counter: 0,

            view: TMat4::identity(),
            vp: Uniform::new(allocators, TMat4::identity()).unwrap(),

            command_buffer: None,
            swapchain_index: None,
            swapchain_future: None,

            used_resources: LinkedList::new(),
        }
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    pub fn flush(&mut self) -> Result<(), Validated<VulkanError>> {
        for (_, frame) in self.frames_in_flight.drain() {
            frame.future.0.wait(None)?;
        }
        Ok(())
    }

    pub fn end_frame(&mut self) {
        self.frame_counter += 1;
    }

    pub fn start_frame(&mut self) {
        let mut swapchain = self.swapchain.write();

        if swapchain.recreate_swapchain {
            swapchain.create_swapchain();
        }

        self.vp.data = self.view * swapchain.projection;
        self.vp.upload().unwrap();

        let (swapchain_index, swapchain_future) = swapchain.acquire_image();
        self.swapchain_index = Some(swapchain_index);
        self.swapchain_future = Some(MainRenderThread(swapchain_future));

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.allocators.read().command_buffer_allocator,
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
            .set_viewport(0, vec![swapchain.viewport.clone()].into())
            .unwrap();

        self.command_buffer = Some(MainRenderThread(commands));
    }
}
