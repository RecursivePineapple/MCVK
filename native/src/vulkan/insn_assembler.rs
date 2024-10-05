use std::array::from_fn;
use std::{collections::HashMap, sync::Arc};

use fastset::Set;
use nalgebra::{Orthographic3, UnitQuaternion};
use nalgebra_glm::{DVec3, TMat4, Vec3, Vec4};

use vulkano::format::Format;
use vulkano::pipeline::graphics::vertex_input::{
    VertexBufferDescription, VertexInputRate, VertexMemberInfo,
};

use super::commands::CommandQueue;
use super::dynamic_shader::{
    ColorMode, DynamicPipelinePushConstants, DynamicPipelineSpec, ShaderMatrixMode, VertexInputSpec,
};
use super::sandbox_jni::jni_prelude::DrawMode;
use super::{
    commands::RenderCommand,
    sandbox::{MatrixMode, OrthoData, PointerArrayType, PointerDataType, RenderInstruction},
};

#[derive(Debug)]
struct MatrixStack {
    pub top: usize,
    pub matrices: Vec<TMat4<f64>>,
}

impl MatrixStack {
    pub fn new() -> Self {
        Self {
            top: 0,
            matrices: vec![TMat4::identity()],
        }
    }

    pub fn push(&mut self) {
        self.top += 1;
        if self.top > self.matrices.len() {
            self.matrices.push(self.matrices[self.top - 1]);
        } else {
            self.matrices[self.top] = self.matrices[self.top - 1];
        }
    }

    pub fn pop(&mut self) {
        if self.top > 0 {
            self.top -= 1;
        }
    }

    pub fn get(&self) -> &TMat4<f64> {
        &self.matrices[self.top]
    }

    pub fn load_identity(&mut self) {
        self.matrices[self.top].fill_with_identity();
    }

    pub fn translate(&mut self, v: &Vec3) {
        self.matrices[self.top].append_translation_mut(&v.cast::<f64>());
    }

    pub fn translated(&mut self, v: &DVec3) {
        self.matrices[self.top].append_translation_mut(&v);
    }

    pub fn scale(&mut self, v: &Vec3) {
        self.matrices[self.top].append_nonuniform_scaling_mut(&v.cast::<f64>());
    }

    pub fn scaled(&mut self, v: &DVec3) {
        self.matrices[self.top].append_nonuniform_scaling_mut(&v);
    }

    pub fn rotate(&mut self, axis: &Vec3, angle: f32) {
        let q = UnitQuaternion::new(axis.normalize() * angle);
        let qm: TMat4<f32> = q.to_rotation_matrix().into();
        self.matrices[self.top] = qm.cast::<f64>() * self.matrices[self.top];
    }

    pub fn rotated(&mut self, axis: &DVec3, angle: f64) {
        let q = UnitQuaternion::new(axis.normalize() * angle);
        let qm: TMat4<f64> = q.to_rotation_matrix().into();
        self.matrices[self.top] = qm * self.matrices[self.top];
    }

    pub fn ortho(&mut self, params: &OrthoData) {
        let m = Orthographic3::new(
            params.left,
            params.right,
            params.bottom,
            params.top,
            params.z_near,
            params.z_far,
        );
        self.matrices[self.top] = m.as_matrix().cast::<f64>() * self.matrices[self.top];
    }
}

#[derive(Debug)]
struct ClientArray {
    pub enabled: bool,
    pub vec_count: u32,
    pub item_type: PointerDataType,
    pub size: u8,
    pub data: Option<Arc<Vec<u8>>>,
}

impl ClientArray {
    pub fn new() -> Self {
        Self {
            enabled: false,
            vec_count: 0,
            item_type: PointerDataType::U8,
            size: 0,
            data: None,
        }
    }
}

const MODELVIEW_MATRIX_IDX: usize = 0;
const PROJECTION_MATRIX_IDX: usize = 1;
const TEXTURE_MATRIX_IDX: usize = 2;
const COLOR_MATRIX_IDX: usize = 3;

fn get_matrix_index(mode: &MatrixMode) -> usize {
    match mode {
        MatrixMode::ModelView => MODELVIEW_MATRIX_IDX,
        MatrixMode::Projection => PROJECTION_MATRIX_IDX,
        MatrixMode::Texture => TEXTURE_MATRIX_IDX,
        MatrixMode::Color => COLOR_MATRIX_IDX,
    }
}

const COLOR_ARRAY_IDX: usize = 0;
const EDGEFLAG_ARRAY_IDX: usize = 1;
const FOGCOORD_ARRAY_IDX: usize = 2;
const COLORINDEX_ARRAY_IDX: usize = 3;
const NORMAL_ARRAY_IDX: usize = 4;
const SECONDARYCOLOR_ARRAY_IDX: usize = 5;
const TEXCOORD_ARRAY_IDX: usize = 6;
const VERTEX_ARRAY_IDX: usize = 7;

fn get_client_array_index(array: &PointerArrayType) -> usize {
    match array {
        PointerArrayType::Color => COLOR_ARRAY_IDX,
        PointerArrayType::EdgeFlag => EDGEFLAG_ARRAY_IDX,
        PointerArrayType::FogCoord => FOGCOORD_ARRAY_IDX,
        PointerArrayType::ColorIndex => COLORINDEX_ARRAY_IDX,
        PointerArrayType::Normal => NORMAL_ARRAY_IDX,
        PointerArrayType::SecondaryColor => SECONDARYCOLOR_ARRAY_IDX,
        PointerArrayType::TexCoord => TEXCOORD_ARRAY_IDX,
        PointerArrayType::Vertex => VERTEX_ARRAY_IDX,
    }
}

fn get_client_array_type(index: usize) -> PointerArrayType {
    match index {
        COLOR_ARRAY_IDX => PointerArrayType::Color,
        EDGEFLAG_ARRAY_IDX => PointerArrayType::EdgeFlag,
        FOGCOORD_ARRAY_IDX => PointerArrayType::FogCoord,
        COLORINDEX_ARRAY_IDX => PointerArrayType::ColorIndex,
        NORMAL_ARRAY_IDX => PointerArrayType::Normal,
        SECONDARYCOLOR_ARRAY_IDX => PointerArrayType::SecondaryColor,
        TEXCOORD_ARRAY_IDX => PointerArrayType::TexCoord,
        VERTEX_ARRAY_IDX => PointerArrayType::Vertex,
        _ => panic!(),
    }
}

fn get_client_array_name(index: usize) -> &'static str {
    match index {
        COLOR_ARRAY_IDX => "color",
        EDGEFLAG_ARRAY_IDX => "edgeflag",
        FOGCOORD_ARRAY_IDX => "fogcoord",
        COLORINDEX_ARRAY_IDX => "color_index",
        NORMAL_ARRAY_IDX => "normal",
        SECONDARYCOLOR_ARRAY_IDX => "secondary_color",
        TEXCOORD_ARRAY_IDX => "texcoord",
        VERTEX_ARRAY_IDX => "pos",
        _ => panic!(),
    }
}

impl RenderInstruction {
    pub fn is_matrix_mutation(&self) -> bool {
        match self {
            RenderInstruction::PushMatrix => true,
            RenderInstruction::PopMatrix => true,
            RenderInstruction::LoadIdentity => true,
            RenderInstruction::Ortho { .. } => true,
            RenderInstruction::Translate { .. } => true,
            RenderInstruction::Translated { .. } => true,
            RenderInstruction::Rotate { .. } => true,
            RenderInstruction::Rotated { .. } => true,
            RenderInstruction::Scale { .. } => true,
            RenderInstruction::Scaled { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct VertexBufferSlot<'a> {
    pub array: &'a ClientArray,
    pub array_type: PointerArrayType,
    pub buffer_offset: u32,
}

#[derive(Debug)]
struct TextureUnit {
    pub bound_texture: Option<i32>,
}

impl TextureUnit {
    pub fn new() -> Self {
        Self {
            bound_texture: None,
        }
    }
}

const MAX_TEXTURE_UNITS: usize = 16;

#[derive(Debug)]
pub struct RenderInsnAssembler {
    active_flags: Set,

    active_matrix: usize,
    matrix_stacks: [MatrixStack; 4],
    active_mvp_cache: Option<TMat4<f64>>,

    active_unit: usize,
    texture_units: [TextureUnit; MAX_TEXTURE_UNITS],

    active_color: Vec4,
    texcoord: Vec4,

    client_arrays: [ClientArray; 8],

    pub commands: CommandQueue,
}

impl RenderInsnAssembler {
    pub fn new(commands: CommandQueue) -> Self {
        Self {
            active_flags: Set::with_capacity(64),

            active_matrix: 0,
            matrix_stacks: from_fn(|_| MatrixStack::new()),
            active_mvp_cache: None,

            active_unit: 0,
            texture_units: from_fn(|_| TextureUnit::new()),

            active_color: [1.0; 4].into(),
            texcoord: [0.0; 4].into(),

            client_arrays: from_fn(|_| ClientArray::new()),

            commands,
        }
    }

    pub fn feed(&mut self, insns: &[RenderInstruction]) {
        for insn in insns {
            if insn.is_matrix_mutation()
                && (self.active_matrix == PROJECTION_MATRIX_IDX
                    || self.active_matrix == MODELVIEW_MATRIX_IDX)
            {
                self.active_mvp_cache.take();
            }

            match insn {
                RenderInstruction::MatrixMode(mode) => {
                    self.active_matrix = get_matrix_index(mode);
                }
                RenderInstruction::PushMatrix => {
                    self.matrix_stacks[self.active_matrix].push();
                }
                RenderInstruction::PopMatrix => {
                    self.matrix_stacks[self.active_matrix].pop();
                }
                RenderInstruction::LoadIdentity => {
                    self.matrix_stacks[self.active_matrix].load_identity();
                }
                RenderInstruction::Ortho { data } => {
                    self.matrix_stacks[self.active_matrix].ortho(&*data);
                }
                RenderInstruction::Translate { delta } => {
                    self.matrix_stacks[self.active_matrix].translate(delta);
                }
                RenderInstruction::Translated { delta } => {
                    self.matrix_stacks[self.active_matrix].translated(delta);
                }
                RenderInstruction::Rotate { angle, axis } => {
                    self.matrix_stacks[self.active_matrix].rotate(axis, *angle);
                }
                RenderInstruction::Rotated { angle, axis } => {
                    self.matrix_stacks[self.active_matrix].rotated(axis, *angle);
                }
                RenderInstruction::Scale { scale } => {
                    self.matrix_stacks[self.active_matrix].scale(scale);
                }
                RenderInstruction::Scaled { scale } => {
                    self.matrix_stacks[self.active_matrix].scaled(scale);
                }

                RenderInstruction::Enable(param) => {
                    self.active_flags.insert(*param as usize);
                }
                RenderInstruction::Disable(param) => {
                    let param = *param as usize;
                    self.active_flags.remove(&param);
                }

                RenderInstruction::SetClientState {
                    enabled,
                    array_type,
                } => {
                    self.client_arrays[get_client_array_index(array_type)].enabled = *enabled;
                }
                RenderInstruction::SetPointer {
                    size,
                    vec_count,
                    array_type,
                    item_type,
                    data,
                } => {
                    let array = &mut self.client_arrays[get_client_array_index(array_type)];
                    array.size = *size;
                    array.vec_count = *vec_count;
                    array.item_type = item_type.clone();
                    array.data = Some(data.clone());
                }
                RenderInstruction::DrawArrays { mode, first, count } => {
                    self.draw_arrays(mode.clone(), *first, *count);
                }

                RenderInstruction::SetActiveTextureUnit(unit) => {
                    self.active_unit = *unit;
                }
                RenderInstruction::BindTexture(id) => {
                    if *id == 0 {
                        self.texture_units[self.active_unit].bound_texture = None;
                    } else {
                        self.texture_units[self.active_unit].bound_texture = Some(*id);
                    }
                }

                RenderInstruction::TexCoord(coord) => {
                    self.texcoord = coord.clone();
                }

                RenderInstruction::SetColor(color) => {
                    self.active_color = color.clone();
                }

                RenderInstruction::Begin(mode) => todo!(),
                RenderInstruction::Vertex(v) => todo!(),
                RenderInstruction::End => todo!(),

                RenderInstruction::AlphaFunc => todo!(),

                RenderInstruction::ClearDepth => {
                    self.commands.push(RenderCommand::ClearDepth);
                }
            }
        }
    }

    fn get_mvp_matrix(&mut self) -> TMat4<f64> {
        if let Some(mat) = self.active_mvp_cache.as_ref() {
            return mat.clone();
        }

        let proj = self.matrix_stacks[PROJECTION_MATRIX_IDX].get();
        let mv = self.matrix_stacks[MODELVIEW_MATRIX_IDX].get();

        self.active_mvp_cache = Some(proj * mv);

        self.active_mvp_cache.as_ref().unwrap().clone()
    }

    fn get_vertex_buffer_layout(
        &mut self,
    ) -> (VertexBufferDescription, Vec<VertexBufferSlot>, Option<u32>) {
        let mut desc = VertexBufferDescription {
            members: HashMap::new(),
            stride: 0,
            input_rate: VertexInputRate::Vertex,
        };

        let mut layout = Vec::new();

        let mut vertex_count = None;

        for (i, array) in self.client_arrays.iter().enumerate() {
            if array.enabled {
                if array.data.is_none() {
                    panic!("no data set but the array is enabled"); // TODO: don't panic here
                }

                let size = array.item_type.size() as u32;
                let n_past_align = desc.stride % size;

                if let Some(vc) = &vertex_count {
                    if array.vec_count != *vc {
                        tracing::warn!(
                            what = "found client array length mismatch",
                            array = get_client_array_name(i),
                            array_length = array.vec_count,
                            expected_length = *vc,
                        );
                        if array.vec_count < *vc {
                            vertex_count = Some(array.vec_count);
                        }
                    }
                } else {
                    vertex_count = Some(array.vec_count);
                }

                desc.stride += n_past_align;

                desc.members.insert(
                    get_client_array_name(i).to_owned(),
                    VertexMemberInfo {
                        offset: desc.stride as usize,
                        format: match array.item_type {
                            PointerDataType::F64 => Format::R64_SFLOAT,
                            _ => Format::R32_SFLOAT,
                        },
                        num_elements: array.size as u32,
                    },
                );

                layout.push(VertexBufferSlot {
                    array,
                    buffer_offset: desc.stride,
                    array_type: get_client_array_type(i),
                });

                desc.stride += size * array.size as u32;
            }
        }

        (desc, layout, vertex_count)
    }

    pub fn draw_arrays(&mut self, mode: DrawMode, first: u32, count: u32) {
        let (desc, layout, vertex_count) = self.get_vertex_buffer_layout();
        let vertex_count = match vertex_count {
            Some(v) => v as usize,
            None => {
                return;
            }
        };

        let mut buffer = Vec::new();
        buffer.resize(vertex_count * (desc.stride as usize), 0);

        for i in 0..vertex_count {
            for array in &layout {
                let dest_start = i * desc.stride as usize + array.buffer_offset as usize;
                let vec_byte_size = array.array.size as usize * array.array.item_type.size();
                let src_start = i * vec_byte_size;

                let dest = &mut buffer[dest_start..dest_start + vec_byte_size];
                let src = &array.array.data.as_ref().unwrap()[src_start..src_start + vec_byte_size];

                assert_eq!(dest.len(), src.len());

                macro_rules! convert {
                    ($from:path) => {{
                        let (_, dest, _) = unsafe { dest.align_to_mut::<f32>() };
                        let (_, src, _) = unsafe { src.align_to::<$from>() };

                        assert_eq!(dest.len(), src.len());

                        for i in 0..dest.len() {
                            dest[i] = src[i] as f32;
                        }
                    }};
                }

                macro_rules! convert_norm {
                    ($from:path) => {{
                        let (_, dest, _) = unsafe { dest.align_to_mut::<f32>() };
                        let (_, src, _) = unsafe { src.align_to::<$from>() };

                        assert_eq!(dest.len(), src.len());

                        for i in 0..dest.len() {
                            dest[i] = (src[i] as f32) / (<$from>::MAX as f32);
                        }
                    }};
                }

                if array.array_type == PointerArrayType::Color
                    || array.array_type == PointerArrayType::SecondaryColor
                {
                    match array.array.item_type {
                        PointerDataType::U8 => convert_norm!(u8),
                        PointerDataType::I8 => convert_norm!(i8),
                        PointerDataType::U16 => convert_norm!(u16),
                        PointerDataType::I16 => convert_norm!(i16),
                        PointerDataType::U32 => convert_norm!(u32),
                        PointerDataType::I32 => convert_norm!(i32),
                        PointerDataType::F32 | PointerDataType::F64 => {
                            dest.copy_from_slice(src);
                        }
                    }
                } else {
                    match array.array.item_type {
                        PointerDataType::U8 => convert!(u8),
                        PointerDataType::I8 => convert!(i8),
                        PointerDataType::U16 => convert!(u16),
                        PointerDataType::I16 => convert!(i16),
                        PointerDataType::U32 => convert!(u32),
                        PointerDataType::I32 => convert!(i32),
                        PointerDataType::F32 | PointerDataType::F64 => {
                            dest.copy_from_slice(src);
                        }
                    }
                }
            }
        }

        let pipeline = DynamicPipelineSpec {
            color: ColorMode::Flat_PC,
            matrix: ShaderMatrixMode::MVP_PC,
            normal: None,
            position: VertexInputSpec {
                name: "position".to_owned(),
                format: vulkano::format::Format::R32_SFLOAT,
                num_elements: 3,
            },
        };

        let mut push_constants = Vec::new();

        push_constants.push(DynamicPipelinePushConstants::MVP(self.get_mvp_matrix()));

        if matches!(pipeline.color, ColorMode::Flat_PC) {
            push_constants.push(DynamicPipelinePushConstants::Color(
                self.active_color.clone().into(),
            ));
        }

        self.commands
            .push(RenderCommand::BindDynamicGraphicsPipeline {
                pipeline,
                push_constants,
            });

        self.commands.push(RenderCommand::Draw {
            mode: mode.clone(),
            vertex: desc,
            start_vertex: first,
            vertex_count: count,
            data: Arc::new(buffer),
        });
    }

    pub fn get_active_texture(&self) -> Option<i32> {
        self.texture_units[self.active_unit].bound_texture.clone()
    }
}
