use std::collections::HashMap;
use std::hash::Hash;
use std::mem::variant_count;
use std::sync::Arc;
use std::sync::Weak;

use concat_string::concat_string;
use derivative::Derivative;
use glslang::Compiler;
use glslang::CompilerOptions;
use glslang::Program;
use glslang::ShaderInput;
use glslang::ShaderSource;
use lru::LruCache;
use nalgebra_glm::TMat4;
use nalgebra_glm::Vec4;
use num::ToPrimitive;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::layout::DescriptorSetLayoutBinding;
use vulkano::descriptor_set::layout::DescriptorSetLayoutCreateInfo;
use vulkano::descriptor_set::layout::DescriptorType;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::depth_stencil::CompareOp;
use vulkano::pipeline::graphics::depth_stencil::DepthState;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::rasterization::CullMode;
use vulkano::pipeline::graphics::rasterization::FrontFace;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineSubpassType;
use vulkano::pipeline::graphics::vertex_input::VertexInputAttributeDescription;
use vulkano::pipeline::graphics::vertex_input::VertexInputBindingDescription;
use vulkano::pipeline::graphics::vertex_input::VertexInputRate;
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::pipeline::graphics::viewport::Scissor;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineLayoutCreateInfo;
use vulkano::pipeline::layout::PushConstantRange;
use vulkano::pipeline::DynamicState;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::PipelineLayout;
use vulkano::pipeline::PipelineShaderStageCreateInfo;
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::shader::ShaderModuleCreateInfo;
use vulkano::shader::ShaderStages;
use weak_table::WeakValueHashMap;

use super::sandbox::DrawMode;
use super::sandbox::GLDataType;
use super::sandbox::PointerArrayType;
use super::swapchain::SwapchainManager;
use super::utils::Ref;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Hash, ToPrimitive, FromPrimitive)]
pub enum VertexInputType {
    Position = 0,
    Normal = 1,
    Color = 2,
    TexCoord = 3,
    TexIndex = 4,
}

impl From<PointerArrayType> for VertexInputType {
    fn from(value: PointerArrayType) -> Self {
        match value {
            PointerArrayType::Color => Self::Color,
            PointerArrayType::EdgeFlag => panic!(),
            PointerArrayType::FogCoord => panic!(),
            PointerArrayType::ColorIndex => panic!(),
            PointerArrayType::Normal => Self::Normal,
            PointerArrayType::SecondaryColor => panic!(),
            PointerArrayType::TexCoord => Self::TexCoord,
            PointerArrayType::Vertex => Self::Position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct VertexInputSpec {
    pub data_type: GLDataType,
    pub num_elements: u8,
    pub offset: u8,
}

impl VertexInputSpec {
    pub fn as_vector(&self) -> VectorDataType {
        if self.num_elements > 4 {
            panic!();
        }

        match self.data_type {
            GLDataType::U8 => todo!(),
            GLDataType::I8 => todo!(),
            GLDataType::U16 => todo!(),
            GLDataType::I16 => todo!(),
            GLDataType::U32 => todo!(),
            GLDataType::I32 => todo!(),
            GLDataType::F32 => todo!(),
            GLDataType::F64 => todo!(),
        }
    }
}

pub type VertexBufferFields = [Option<VertexInputSpec>; variant_count::<VertexInputType>()];

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct VertexBufferLayout {
    pub fields: VertexBufferFields,
    pub stride: u8,
}

impl VertexBufferLayout {
    pub fn align_to(&mut self, width: u8) {
        let n_past_align = self.stride % width;
        if n_past_align > 0 {
            self.stride += width - n_past_align;
        }
    }

    pub fn position(&self) -> Option<&VertexInputSpec> {
        self.fields[VertexInputType::Position.to_usize().unwrap()].as_ref()
    }

    pub fn normal(&self) -> Option<&VertexInputSpec> {
        self.fields[VertexInputType::Normal.to_usize().unwrap()].as_ref()
    }

    pub fn color(&self) -> Option<&VertexInputSpec> {
        self.fields[VertexInputType::Color.to_usize().unwrap()].as_ref()
    }

    pub fn texcoord(&self) -> Option<&VertexInputSpec> {
        self.fields[VertexInputType::TexCoord.to_usize().unwrap()].as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum DataSource {
    PushConstant,
    Uniform { set: u8, binding: u8 },
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ShaderMatrixMode {
    /// P * V * M in a mat4
    MVP(DataSource),
    /// P * V, M in two mat4s
    /// Both DataSources should be the same type
    VP_M(DataSource, DataSource),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ColorMode {
    Flat(DataSource),
    Texture { set: u8, binding: u8 },
    Array,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DynamicPipelinePushConstants {
    pub mvp: Option<TMat4<f32>>,
    pub color: Option<Vec4>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DynamicPipelineSpec {
    pub draw_mode: DrawMode,

    pub vertex_buffer: VertexBufferLayout,
    pub color: ColorMode,

    pub matrix: ShaderMatrixMode,

    pub rasterization: DynamicPipelineRasterization,
}

impl DynamicPipelineSpec {
    pub fn position(&self) -> &VertexInputSpec {
        self.vertex_buffer.position().unwrap()
    }

    pub fn normal(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.normal()
    }

    pub fn texcoord(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.texcoord()
    }

    pub fn color(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.color()
    }
}

/// The set of fields relevant to shaders
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShaderSpec {
    pub vertex_buffer: VertexBufferLayout,
    pub color: ColorMode,

    pub matrix: ShaderMatrixMode,
}

impl From<&DynamicPipelineSpec> for ShaderSpec {
    fn from(value: &DynamicPipelineSpec) -> Self {
        Self {
            vertex_buffer: value.vertex_buffer.clone(),
            color: value.color.clone(),
            matrix: value.matrix.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicPipelineRasterization {
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    /// pre-multiplied by 10
    pub line_width: u32,
    pub color_blending: Option<AttachmentBlend>,
}

impl Hash for DynamicPipelineRasterization {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cull_mode.hash(state);
        self.front_face.hash(state);
        (self.line_width as i32).hash(state);
        self.color_blending.is_some().hash(state);
        if let Some(blending) = self.color_blending.as_ref() {
            blending.src_color_blend_factor.hash(state);
            blending.dst_color_blend_factor.hash(state);
            blending.color_blend_op.hash(state);
            blending.src_alpha_blend_factor.hash(state);
            blending.dst_alpha_blend_factor.hash(state);
            blending.alpha_blend_op.hash(state);
        }
    }
}

impl Default for DynamicPipelineRasterization {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            line_width: 10,
            color_blending: Some(AttachmentBlend::ignore_source()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VectorDataType {
    U8(u8),
    I8(u8),
    U16(u8),
    I16(u8),
    U32(u8),
    I32(u8),
    F32(u8),
    F64(u8),
}

impl VectorDataType {
    pub fn ordinal(&self) -> GLDataType {
        match self {
            Self::U8(_) => GLDataType::U8,
            Self::I8(_) => GLDataType::I8,
            Self::U16(_) => GLDataType::U16,
            Self::I16(_) => GLDataType::I16,
            Self::U32(_) => GLDataType::U32,
            Self::I32(_) => GLDataType::I32,
            Self::F32(_) => GLDataType::F32,
            Self::F64(_) => GLDataType::F64,
        }
    }

    pub fn size(&self) -> u8 {
        match self {
            Self::U8(size)
            | Self::I8(size)
            | Self::U16(size)
            | Self::I16(size)
            | Self::U32(size)
            | Self::I32(size)
            | Self::F32(size)
            | Self::F64(size) => *size,
        }
    }

    pub fn get_widening_zeroes(&self) -> &'static str {
        match self.size() {
            1 => ", 0.0, 0.0, 0.0",
            2 => ", 0.0, 0.0",
            3 => ", 0.0",
            _ => "",
        }
    }

    pub fn as_strs(&self) -> (&'static str, &'static str) {
        let size = self.size();

        if size <= 1 {
            match self {
                Self::U8(_) | Self::U16(_) | Self::U32(_) => ("uint", ""),
                Self::I8(_) | Self::I16(_) | Self::I32(_) => ("int", ""),
                Self::F32(_) => ("float", ""),
                Self::F64(_) => ("double", ""),
            }
        } else {
            let suffix = match size {
                2 => "2",
                3 => "3",
                4 => "4",
                _ => panic!("illegal shader input size {size}"),
            };

            match self {
                Self::U8(_) | Self::U16(_) | Self::U32(_) => ("uvec", suffix),
                Self::I8(_) | Self::I16(_) | Self::I32(_) => ("ivec", suffix),
                Self::F32(_) => ("vec", suffix),
                Self::F64(_) => ("dvec", suffix),
            }
        }
    }

    pub fn as_format(&self) -> Format {
        match self {
            VectorDataType::F32(1) => Format::R32_SFLOAT,
            VectorDataType::F32(2) => Format::R32G32_SFLOAT,
            VectorDataType::F32(3) => Format::R32G32B32_SFLOAT,
            VectorDataType::F32(4) => Format::R32G32B32A32_SFLOAT,
            VectorDataType::F64(1) => Format::R64_SFLOAT,
            VectorDataType::F64(2) => Format::R64G64_SFLOAT,
            VectorDataType::F64(3) => Format::R64G64B64_SFLOAT,
            VectorDataType::F64(4) => Format::R64G64B64A64_SFLOAT,
            _ => panic!(),
        }
    }
}

impl ShaderSpec {
    fn append_input(code: &mut String, location: u32, var_type: &VectorDataType, name: &str) {
        Self::append_io(code, location, true, var_type, name);
    }

    fn append_output(code: &mut String, location: u32, var_type: &VectorDataType, name: &str) {
        Self::append_io(code, location, false, var_type, name);
    }

    fn append_io(
        code: &mut String,
        location: u32,
        is_input: bool,
        var_type: &VectorDataType,
        name: &str,
    ) {
        let (tprefix, tsuffix) = var_type.as_strs();
        *code += &concat_string!(
            "layout(location = ",
            location.to_string(),
            if is_input { ") in " } else { ") out " },
            tprefix,
            tsuffix,
            " ",
            name,
            ";\n"
        );
    }

    fn position(&self) -> &VertexInputSpec {
        self.vertex_buffer.position().unwrap()
    }

    fn normal(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.normal()
    }

    fn texcoord(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.texcoord()
    }

    fn color(&self) -> Option<&VertexInputSpec> {
        self.vertex_buffer.color()
    }

    pub fn get_vertex_shader_code(&self) -> String {
        let mut code = String::with_capacity(1024);

        code += "#version 450\n";

        // VERTEX INPUTS

        Self::append_input(&mut code, 0, &self.position().as_vector(), "position_in");

        if let Some(normal) = self.normal() {
            Self::append_input(&mut code, 1, &normal.as_vector(), "normal_in");
        }

        match &self.color {
            ColorMode::Flat(DataSource::PushConstant) => {}
            ColorMode::Flat(DataSource::Uniform { .. }) => {}
            ColorMode::Texture { .. } => {
                Self::append_input(
                    &mut code,
                    2,
                    &self.texcoord().unwrap().as_vector(),
                    "texcoord_in",
                );
            }
            ColorMode::Array => {
                Self::append_input(
                    &mut code,
                    2,
                    &self.color().unwrap().as_vector(),
                    "colors_in",
                );
            }
        }

        // PUSH CONSTANTS

        code += "layout(push_constant) uniform constants {\n";

        match &self.matrix {
            ShaderMatrixMode::MVP(DataSource::PushConstant) => {
                code += "  mat4 mvp\n";
            }
            ShaderMatrixMode::VP_M(DataSource::PushConstant, DataSource::PushConstant) => {
                code += "  mat4 model;\n";
                code += "  mat4 vp;\n";
            }
            _ => {}
        }

        if let ColorMode::Flat(DataSource::PushConstant) = &self.color {
            code += "  vec4 color;\n";
        }

        code += "} PushConstants;\n";

        // UNIFORMS

        match &self.matrix {
            ShaderMatrixMode::MVP(DataSource::Uniform { set, binding }) => {
                code += &format!("layout (set = {set}, binding = {binding}) uniform MVPUniformData {{ mat4 matrix; }} MVPUniform;\n");
            }
            ShaderMatrixMode::VP_M(
                DataSource::Uniform {
                    set: vp_set,
                    binding: vp_binding,
                },
                DataSource::Uniform {
                    set: model_set,
                    binding: model_binding,
                },
            ) => {
                code += &format!("layout (set = {vp_set}, binding = {vp_binding}) uniform VPUniformData {{ mat4 matrix; }} VPUniform;\n");
                code += &format!("layout (set = {model_set}, binding = {model_binding}) uniform MUniformData {{ mat4 matrix; }} MUniform;\n");
            }
            _ => {}
        }

        match &self.color {
            ColorMode::Flat(DataSource::Uniform { set, binding }) => {
                code += &format!("layout (set = {set}, binding = {binding}) uniform ColorUniformData {{ vec4 color; }} ColorUniform;");
            }
            _ => {}
        }

        // OUTPUTS TO FRAG SHADER

        match &self.color {
            ColorMode::Array => {
                Self::append_output(&mut code, 0, &VectorDataType::F32(4), "frag_color_out");
            }
            ColorMode::Texture { .. } => {
                Self::append_output(&mut code, 0, &VectorDataType::F32(2), "tex_coord_out");
            }
            _ => {}
        }

        if self.normal().is_some() {
            Self::append_output(&mut code, 1, &VectorDataType::F32(3), "normal_out");
        }

        // CODE

        code += "void main() {\n";

        match &self.matrix {
            ShaderMatrixMode::MVP(DataSource::PushConstant) => {
                code += &concat_string!(
                    "  gl_Position = vec4(PushConstants.mvp * position_in",
                    self.position().as_vector().get_widening_zeroes(),
                    ");\n"
                );
            }
            ShaderMatrixMode::MVP(DataSource::Uniform { .. }) => {
                code += &concat_string!(
                    "  gl_Position = vec4(MVPUniform.mvp * position_in",
                    self.position().as_vector().get_widening_zeroes(),
                    ");\n"
                );
            }
            ShaderMatrixMode::VP_M(DataSource::PushConstant, DataSource::PushConstant) => {
                code += &concat_string!(
                    "  gl_Position = vec4(PushConstants.model * PushConstants.vp * position_in",
                    self.position().as_vector().get_widening_zeroes(),
                    ");\n"
                );
            }
            ShaderMatrixMode::VP_M(DataSource::Uniform { .. }, DataSource::Uniform { .. }) => {
                code += &concat_string!(
                    "  gl_Position = vec4(MUniform.matrix * VPUniform.matrix * position_in",
                    self.position().as_vector().get_widening_zeroes(),
                    ");\n"
                );
            }
            _ => {}
        }

        match &self.color {
            ColorMode::Flat(DataSource::PushConstant) => {
                code += "  frag_color_out = PushConstants.color;\n";
            }
            ColorMode::Flat(DataSource::Uniform { .. }) => {
                code += "  frag_color_out = ColorUniform.color;\n";
            }
            ColorMode::Texture { .. } => {
                code += &format!("  texcoord_out = texcoord_in;\n");
            }
            ColorMode::Array => {
                code += &format!("  frag_color_out = color_in;\n");
            }
        }

        if self.normal().is_some() {
            code += &format!("  normal_out = normal_in;\n");
        }

        code += "}\n";

        code.shrink_to_fit();

        code
    }

    pub fn get_fragment_shader_code(&self) -> String {
        let mut code = String::with_capacity(1024);

        code += "#version 450\n";

        // UNIFORMS

        match &self.color {
            ColorMode::Texture { set, binding, .. } => {
                code += &format!(
                    "layout (set = {set}, binding = {binding}) uniform sampler2D sampler;\n"
                );
            }
            _ => {}
        }

        // INPUTS FROM VERT SHADER

        match &self.color {
            ColorMode::Flat(_) | ColorMode::Array => {
                Self::append_input(&mut code, 0, &VectorDataType::F32(4), "frag_color_in");
            }
            ColorMode::Texture { .. } => {
                Self::append_input(&mut code, 0, &VectorDataType::F32(2), "tex_coord_in");
            }
        }

        if self.normal().is_some() {
            Self::append_input(&mut code, 1, &VectorDataType::F32(3), "normal_in");
        }

        // OUTPUTS TO FRAME BUFFERS

        Self::append_output(&mut code, 0, &VectorDataType::F32(4), "frag_color_out");

        if self.normal().is_some() {
            Self::append_input(&mut code, 1, &VectorDataType::F32(3), "normal_out");
        }

        // CODE

        code += "void main() {\n";

        match &self.color {
            ColorMode::Flat(_) | ColorMode::Array => {
                code += "  frag_color_out = frag_color_in;\n";
            }
            ColorMode::Texture { .. } => {
                code += "  frag_color_out = texture(sampler, tex_coord_in);\n";
            }
        }

        if self.normal().is_some() {
            code += &format!("  normal_out = normal_in;\n");
        }

        code += "}\n";

        code.shrink_to_fit();

        code
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DynamicPipeline {
    pub spec: DynamicPipelineSpec,

    pub pipeline: Arc<GraphicsPipeline>,
    pub layout: Arc<PipelineLayout>,
}

pub struct PipelineCompiler {
    pub device: Arc<Device>,
    pub swapchain: Ref<SwapchainManager>,

    cache: WeakValueHashMap<DynamicPipelineSpec, Weak<DynamicPipeline>>,
    vertex_shaders: LruCache<ShaderSpec, Arc<ShaderModule>>,
    fragment_shaders: LruCache<ShaderSpec, Arc<ShaderModule>>,
}

impl PipelineCompiler {
    pub fn compile(&mut self, spec: &DynamicPipelineSpec) -> Arc<DynamicPipeline> {
        if let Some(pipeline) = self.cache.get(spec) {
            return pipeline;
        }

        let mut descriptors = HashMap::new();

        match &spec.color {
            ColorMode::Flat(DataSource::Uniform { set, binding }) => {
                let mut descriptor =
                    DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer);

                descriptor.stages = ShaderStages::VERTEX;

                descriptors.insert((*set, *binding), descriptor);
            }
            ColorMode::Texture { set, binding, .. } => {
                let mut descriptor =
                    DescriptorSetLayoutBinding::descriptor_type(DescriptorType::SampledImage);

                descriptor.stages = ShaderStages::FRAGMENT;

                descriptors.insert((*set, *binding), descriptor);
            }
            _ => {}
        }

        match &spec.matrix {
            ShaderMatrixMode::MVP(DataSource::Uniform { set, binding }) => {
                let mut descriptor =
                    DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer);

                descriptor.stages = ShaderStages::VERTEX;

                descriptors.insert((*set, *binding), descriptor);
            }
            ShaderMatrixMode::VP_M(
                DataSource::Uniform {
                    set: vp_set,
                    binding: vp_binding,
                },
                DataSource::Uniform {
                    set: model_set,
                    binding: model_binding,
                },
            ) => {
                let mut vp_desc =
                    DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer);
                let mut m_desc =
                    DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer);

                vp_desc.stages = ShaderStages::VERTEX;
                m_desc.stages = ShaderStages::VERTEX;

                descriptors.insert((*vp_set, *vp_binding), vp_desc);
                descriptors.insert((*model_set, *model_binding), m_desc);
            }
            _ => {}
        }

        let mut set_layouts = Vec::new();

        for set in 0..descriptors.keys().map(|x| x.0).max().unwrap_or(0) {
            let mut create_info = DescriptorSetLayoutCreateInfo::default();

            for ((_, descriptor), desc_type) in descriptors.iter().filter(|x| x.0 .0 == set) {
                create_info
                    .bindings
                    .insert(*descriptor as u32, desc_type.clone());
            }

            set_layouts.push(DescriptorSetLayout::new(self.device.clone(), create_info).unwrap());
        }

        let mut size = 0;

        match &spec.matrix {
            ShaderMatrixMode::MVP(DataSource::PushConstant) => {
                size += size_of::<TMat4<f32>>();
            }
            ShaderMatrixMode::VP_M(DataSource::PushConstant, DataSource::PushConstant) => {
                size += size_of::<TMat4<f32>>() * 2;
            }
            _ => {}
        }

        if let ColorMode::Flat(DataSource::PushConstant) = &spec.color {
            size += size_of::<Vec4>();
        }

        let layout = PipelineLayout::new(
            self.device.clone(),
            PipelineLayoutCreateInfo {
                set_layouts,
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::all_graphics(),
                    offset: 0,
                    size: size as u32,
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let mut vertex_input = VertexInputState::new()
            .binding(
                0,
                VertexInputBindingDescription {
                    stride: spec.vertex_buffer.stride as u32,
                    input_rate: VertexInputRate::Vertex,
                },
            )
            .attribute(
                0,
                VertexInputAttributeDescription {
                    binding: 0,
                    format: spec.position().as_vector().as_format(),
                    offset: spec.position().offset as u32,
                },
            );

        if let Some(normal) = spec.normal() {
            vertex_input = vertex_input.attribute(
                1,
                VertexInputAttributeDescription {
                    binding: 0,
                    format: normal.as_vector().as_format(),
                    offset: normal.offset as u32,
                },
            );
        }

        match &spec.color {
            ColorMode::Texture { .. } => {
                let texcoord = spec.texcoord().unwrap();
                vertex_input = vertex_input.attribute(
                    2,
                    VertexInputAttributeDescription {
                        binding: 0,
                        format: texcoord.as_vector().as_format(),
                        offset: texcoord.offset as u32,
                    },
                );
            }
            ColorMode::Array => {
                let color = spec.color().unwrap();
                vertex_input = vertex_input.attribute(
                    2,
                    VertexInputAttributeDescription {
                        binding: 0,
                        format: color.as_vector().as_format(),
                        offset: color.offset as u32,
                    },
                );
            }
            _ => {}
        }

        let mut create_info = GraphicsPipelineCreateInfo::layout(layout.clone());

        create_info.vertex_input_state = Some(vertex_input);

        let shader_spec = ShaderSpec::from(spec);

        let vert_shader = self.compile_vertex_shader(&shader_spec);
        let frag_shader = self.compile_fragment_shader(&shader_spec);

        create_info.stages.push(PipelineShaderStageCreateInfo::new(
            vert_shader.entry_point("main").unwrap(),
        ));

        create_info.stages.push(PipelineShaderStageCreateInfo::new(
            frag_shader.entry_point("main").unwrap(),
        ));

        create_info.dynamic_state.insert(DynamicState::Viewport);
        create_info.dynamic_state.insert(DynamicState::Scissor);
        create_info.dynamic_state.insert(DynamicState::LineWidth);
        create_info.dynamic_state.insert(DynamicState::DepthBias);
        create_info.dynamic_state.insert(DynamicState::DepthBounds);
        create_info.dynamic_state.insert(DynamicState::CullMode);
        create_info.dynamic_state.insert(DynamicState::FrontFace);
        create_info.viewport_state = Some(ViewportState {
            viewports: vec![Viewport::default()].into(),
            scissors: vec![Scissor::default()].into(),
            ..Default::default()
        });

        create_info.rasterization_state = Some(RasterizationState {
            cull_mode: spec.rasterization.cull_mode.clone(),
            front_face: spec.rasterization.front_face.clone(),
            line_width: (spec.rasterization.line_width as f32) / 10.0f32,
            ..Default::default()
        });

        create_info.depth_stencil_state = Some(DepthStencilState {
            depth: Some(DepthState {
                write_enable: true,
                compare_op: CompareOp::Less,
            }),
            ..Default::default()
        });

        let render_pass = self.swapchain.read().render_pass.as_ref().unwrap().clone();

        create_info.color_blend_state = Some(ColorBlendState {
            attachments: vec![ColorBlendAttachmentState {
                blend: spec.rasterization.color_blending.clone(),
                ..Default::default()
            }],
            ..Default::default()
        });

        create_info.subpass = Some(PipelineSubpassType::BeginRenderPass(
            Subpass::from(render_pass.clone(), 0).unwrap(),
        ));

        let pipeline = GraphicsPipeline::new(self.device.clone(), None, create_info).unwrap();

        let dyn_pipeline = Arc::new(DynamicPipeline {
            spec: spec.clone(),
            pipeline,
            layout,
        });

        self.cache.insert(spec.clone(), dyn_pipeline.clone());

        dyn_pipeline
    }

    fn compile_vertex_shader(&mut self, spec: &ShaderSpec) -> Arc<ShaderModule> {
        if let Some(module) = self.vertex_shaders.get(spec) {
            return module.clone();
        }

        let compiler = Compiler::acquire().unwrap();

        let source = ShaderSource::try_from(spec.get_vertex_shader_code()).unwrap();

        let input = ShaderInput::new(
            &source,
            glslang::ShaderStage::Vertex,
            &CompilerOptions::default(),
            None,
            None,
        );
        let shader = glslang::Shader::new(&compiler, input.unwrap()).expect("shader init");

        let mut program = Program::new(&compiler);

        program.add_shader(&shader);

        let code = program
            .compile(glslang::ShaderStage::Vertex)
            .expect("shader");

        let module = unsafe {
            ShaderModule::new(self.device.clone(), ShaderModuleCreateInfo::new(&code[..])).unwrap()
        };

        self.vertex_shaders.put(spec.clone(), module.clone());

        module
    }

    fn compile_fragment_shader(&mut self, spec: &ShaderSpec) -> Arc<ShaderModule> {
        if let Some(module) = self.fragment_shaders.get(spec) {
            return module.clone();
        }

        let compiler = Compiler::acquire().unwrap();

        let source = ShaderSource::try_from(spec.get_vertex_shader_code()).unwrap();

        let input = ShaderInput::new(
            &source,
            glslang::ShaderStage::Fragment,
            &CompilerOptions::default(),
            None,
            None,
        );
        let shader = glslang::Shader::new(&compiler, input.unwrap()).expect("shader init");

        let mut program = Program::new(&compiler);

        program.add_shader(&shader);

        let code = program
            .compile(glslang::ShaderStage::Fragment)
            .expect("shader");

        let module = unsafe {
            ShaderModule::new(self.device.clone(), ShaderModuleCreateInfo::new(&code[..])).unwrap()
        };

        self.fragment_shaders.put(spec.clone(), module.clone());

        module
    }
}
