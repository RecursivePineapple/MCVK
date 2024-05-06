use std::sync::Arc;

use derivative::Derivative;
use vulkano::{
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
}
