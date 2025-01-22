use std::sync::Arc;

use derivative::Derivative;
use smallvec::smallvec;
use tokio::sync::mpsc::UnboundedSender;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::allocator::CommandBufferAllocator;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::ClearAttachment;
use vulkano::command_buffer::ClearRect;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::memory::allocator::MemoryTypeFilter;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::GraphicsPipeline;

use super::dynamic_shader::DynamicPipeline;
use super::dynamic_shader::DynamicPipelinePushConstants;
use super::dynamic_shader::DynamicPipelineSpec;
use super::dynamic_shader::PipelineCompiler;
use super::utils::Ref;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderCommand {
    BindDynamicGraphicsPipeline {
        pipeline: DynamicPipelineSpec,
        push_constants: DynamicPipelinePushConstants,
    },
    Draw {
        start_vertex: u32,
        vertex_count: u32,
        data: Arc<Vec<u8>>,
    },
    ClearDepth,
}

#[derive(Debug)]
pub enum CommandQueue {
    Async(UnboundedSender<RenderCommand>),
    Immediate(Box<CommandRecorder<PrimaryAutoCommandBuffer>>),
    /// Only used for tests
    Buffered(Vec<RenderCommand>),
}

impl CommandQueue {
    pub fn push(&mut self, cmd: RenderCommand) -> anyhow::Result<()> {
        match self {
            CommandQueue::Async(queue) => {
                queue.send(cmd)?;
            }
            CommandQueue::Immediate(recorder) => recorder.feed(cmd),
            CommandQueue::Buffered(v) => {
                v.push(cmd);
            }
        }

        Ok(())
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct CommandRecorder<L, A = StandardCommandBufferAllocator>
where
    A: CommandBufferAllocator,
{
    pub allocator: Arc<StandardMemoryAllocator>,

    #[derivative(Debug = "ignore")]
    pub builder: AutoCommandBufferBuilder<L, A>,

    #[derivative(Debug = "ignore")]
    pub pipeline_compiler: Ref<PipelineCompiler>,

    active_dyn_pipeline: Option<(Arc<DynamicPipeline>, DynamicPipelinePushConstants)>,
    active_gfx_pipeline: Option<Arc<GraphicsPipeline>>,
}

impl<L, A> CommandRecorder<L, A>
where
    A: CommandBufferAllocator,
{
    pub fn new(
        allocator: Arc<StandardMemoryAllocator>,
        builder: AutoCommandBufferBuilder<L, A>,
        pipeline_compiler: Ref<PipelineCompiler>,
    ) -> Self {
        Self {
            allocator,
            builder,
            pipeline_compiler,
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
                let different_pipeline = if let Some((active, _)) = &self.active_dyn_pipeline {
                    pipeline != active.spec
                } else {
                    true
                };

                if different_pipeline {
                    let compiled = self.pipeline_compiler.write().compile(&pipeline);
                    self.active_dyn_pipeline = Some((compiled.clone(), Default::default()));

                    self.builder
                        .bind_pipeline_graphics(compiled.pipeline.clone())
                        .unwrap();
                }

                let (pipeline, pc) = self.active_dyn_pipeline.as_ref().unwrap();

                if pc != &push_constants {
                    let mut offset = 0;

                    if let Some(mvp) = push_constants.mvp.as_ref() {
                        self.builder
                            .push_constants(pipeline.layout.clone(), offset, *mvp)
                            .unwrap();
                        offset += size_of_val(mvp) as u32;
                    }

                    if let Some(color) = push_constants.color.as_ref() {
                        self.builder
                            .push_constants(pipeline.layout.clone(), offset, *color)
                            .unwrap();
                    }
                }
            }
            RenderCommand::Draw {
                start_vertex,
                vertex_count,
                data,
            } => {
                let vertex_buffer = vulkano::buffer::Buffer::new_slice::<u8>(
                    self.allocator.clone(),
                    vulkano::buffer::BufferCreateInfo {
                        usage: BufferUsage::VERTEX_BUFFER,
                        ..Default::default()
                    },
                    vulkano::memory::allocator::AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    data.len() as u64,
                )
                .unwrap();

                {
                    let mut guard = vertex_buffer.write().unwrap();
                    guard.copy_from_slice(&data);
                }

                self.builder.bind_vertex_buffers(0, vertex_buffer);
                self.builder.draw(vertex_count, 1, start_vertex, 0);
            }
            RenderCommand::ClearDepth => {
                self.builder.clear_attachments(
                    smallvec![ClearAttachment::Depth(1f32)],
                    smallvec![ClearRect {
                        offset: [0; 2],
                        extent: [u32::MAX; 2],
                        array_layers: 0..0
                    }],
                );
            }
        }
    }
}
