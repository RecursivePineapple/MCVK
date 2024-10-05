use std::sync::Arc;

use derivative::Derivative;
use tokio::sync::mpsc::UnboundedSender;
use vulkano::{
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, BlitImageInfo,
        CopyBufferToImageInfo,
    },
    descriptor_set::DescriptorSetWithOffsets,
    pipeline::{
        graphics::vertex_input::VertexBufferDescription, GraphicsPipeline, PipelineBindPoint,
        PipelineLayout,
    },
};

use super::{
    dynamic_shader::{DynamicPipelinePushConstants, DynamicPipelineSpec},
    sandbox::DrawMode,
};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderCommand {
    BindDynamicGraphicsPipeline {
        pipeline: DynamicPipelineSpec,
        push_constants: Vec<DynamicPipelinePushConstants>,
    },
    BindGraphicsPipeline(#[derivative(Debug = "ignore")] Arc<GraphicsPipeline>),
    BindGraphicsDescriptorSets(
        PipelineBindPoint,
        Arc<PipelineLayout>,
        u32,
        #[derivative(Debug = "ignore")] Vec<DescriptorSetWithOffsets>,
    ),
    Draw {
        mode: DrawMode,
        vertex: VertexBufferDescription,
        start_vertex: u32,
        vertex_count: u32,
        data: Arc<Vec<u8>>,
    },
    UploadImage(CopyBufferToImageInfo, Vec<BlitImageInfo>),
    ClearDepth,
}

#[derive(Debug)]
pub enum CommandQueue {
    Immediate(UnboundedSender<RenderCommand>),
    /// Only used for tests
    Buffered(Vec<RenderCommand>),
}

impl CommandQueue {
    pub fn push(&mut self, cmd: RenderCommand) {
        match self {
            CommandQueue::Immediate(queue) => {
                queue.send(cmd);
            }
            CommandQueue::Buffered(v) => {
                v.push(cmd);
            }
        }
    }
}

pub struct CommandRecorder<L, A>
where
    A: CommandBufferAllocator,
{
    pub builder: AutoCommandBufferBuilder<L, A>,

    active_dyn_pipeline: Option<(Box<DynamicPipelineSpec>, Vec<DynamicPipelinePushConstants>)>,
    active_gfx_pipeline: Option<Arc<GraphicsPipeline>>,
}

impl<L, A> CommandRecorder<L, A>
where
    A: CommandBufferAllocator,
{
    pub fn new(builder: AutoCommandBufferBuilder<L, A>) -> Self {
        Self {
            builder,
            active_dyn_pipeline: None,
            active_gfx_pipeline: None,
        }
    }

    pub fn feed(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::BindDynamicGraphicsPipeline {
                pipeline,
                push_constants,
            } => {
                if let Some((active, pc)) = &self.active_dyn_pipeline {
                    // self.builder
                    //     .push_constants(pipeline_layout, offset, push_constants)
                }
            }
            RenderCommand::BindGraphicsPipeline(_) => todo!(),
            RenderCommand::BindGraphicsDescriptorSets(_, _, _, _) => todo!(),
            RenderCommand::Draw {
                mode,
                vertex,
                start_vertex,
                vertex_count,
                data,
            } => todo!(),
            RenderCommand::UploadImage(_, _) => todo!(),
            RenderCommand::ClearDepth => todo!(),
        }
    }
}
