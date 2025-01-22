use std::array::from_fn;
use std::sync::Arc;

use fastset::Set;
use nalgebra::Orthographic3;
use nalgebra::UnitQuaternion;
use nalgebra_glm::TMat4;
use nalgebra_glm::Vec3;
use nalgebra_glm::Vec4;

use num::ToPrimitive;

use super::commands::CommandQueue;
use super::commands::RenderCommand;
use super::dynamic_shader::ColorMode;
use super::dynamic_shader::DataSource;
use super::dynamic_shader::DynamicPipelinePushConstants;
use super::dynamic_shader::DynamicPipelineRasterization;
use super::dynamic_shader::DynamicPipelineSpec;
use super::dynamic_shader::ShaderMatrixMode;
use super::dynamic_shader::VertexBufferLayout;
use super::dynamic_shader::VertexInputSpec;
use super::dynamic_shader::VertexInputType;
use super::sandbox::GLDataType;
use super::sandbox::MatrixMode;
use super::sandbox::OrthoData;
use super::sandbox::PointerArrayType;
use super::sandbox::RenderInstruction;
use super::sandbox_jni::jni_prelude::DrawMode;
use super::textures::lookup::TextureLookup;

#[derive(Debug)]
struct MatrixStack {
    pub top: usize,
    pub matrices: Vec<TMat4<f32>>,
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

    pub fn get(&self) -> &TMat4<f32> {
        &self.matrices[self.top]
    }

    pub fn load_identity(&mut self) {
        self.matrices[self.top].fill_with_identity();
    }

    pub fn translate(&mut self, v: &Vec3) {
        self.matrices[self.top].append_translation_mut(&v.cast::<f32>());
    }

    pub fn scale(&mut self, v: &Vec3) {
        self.matrices[self.top].append_nonuniform_scaling_mut(&v.cast::<f32>());
    }

    pub fn rotate(&mut self, axis: &Vec3, angle: f32) {
        let q = UnitQuaternion::new(axis.normalize() * angle);
        let qm: TMat4<f32> = q.to_rotation_matrix().into();
        self.matrices[self.top] = qm.cast::<f32>() * self.matrices[self.top];
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
        self.matrices[self.top] = m.as_matrix().cast::<f32>() * self.matrices[self.top];
    }
}

#[derive(Debug)]
struct ClientArray {
    pub enabled: bool,
    pub vertex_count: u32,
    pub data_type: GLDataType,
    pub element_count: u8,
    pub data: Option<Arc<Vec<u8>>>,
}

impl ClientArray {
    pub fn new() -> Self {
        Self {
            enabled: false,
            vertex_count: 0,
            data_type: GLDataType::U8,
            element_count: 0,
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
            RenderInstruction::Rotate { .. } => true,
            RenderInstruction::Scale { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct VertexBufferSlot<'a> {
    pub array: &'a ClientArray,
    pub array_type: PointerArrayType,
    pub data_type: GLDataType,
    pub buffer_offset: u8,
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
    active_mvp_cache: Option<TMat4<f32>>,

    active_unit: usize,
    texture_units: [TextureUnit; MAX_TEXTURE_UNITS],

    active_color: Vec4,
    texcoord: Vec4,

    client_arrays: [ClientArray; 8],

    pub commands: CommandQueue,
    pub texture_lookup: Arc<TextureLookup>,
}

impl RenderInsnAssembler {
    pub fn new(commands: CommandQueue, texture_lookup: Arc<TextureLookup>) -> Self {
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
            texture_lookup,
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
                RenderInstruction::Rotate { angle, axis } => {
                    self.matrix_stacks[self.active_matrix].rotate(axis, *angle);
                }
                RenderInstruction::Scale { scale } => {
                    self.matrix_stacks[self.active_matrix].scale(scale);
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
                    array.element_count = *size;
                    array.vertex_count = *vec_count;
                    array.data_type = item_type.clone();
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
                    self.commands.push(RenderCommand::ClearDepth).unwrap();
                }
            }
        }
    }

    fn get_mvp_matrix(&mut self) -> TMat4<f32> {
        if let Some(mat) = self.active_mvp_cache.as_ref() {
            return mat.clone();
        }

        let proj = self.matrix_stacks[PROJECTION_MATRIX_IDX].get();
        let mv = self.matrix_stacks[MODELVIEW_MATRIX_IDX].get();

        self.active_mvp_cache = Some(proj * mv);

        self.active_mvp_cache.as_ref().unwrap().clone()
    }

    pub fn is_enabled(&self, flag: u32) -> bool {
        self.active_flags.contains(&(flag as usize))
    }

    fn get_vertex_buffer_layout(&self) -> (VertexBufferLayout, Vec<VertexBufferSlot>, usize) {
        let mut desc = VertexBufferLayout {
            fields: [const { None }; _],
            stride: 0,
        };

        let mut layout = Vec::new();

        let mut vertex_count = None;

        for (i, array) in self.client_arrays.iter().enumerate() {
            if !array.enabled {
                continue;
            }

            let array_type = get_client_array_type(i);

            if !array_type.is_supported() {
                tracing::warn!(
                    what = "client array is enabled, but arrays of this type are not supported",
                    array = get_client_array_name(i),
                );
                continue;
            }

            if array.data.is_none() {
                tracing::warn!(
                    what = "client array is enabled, but no data was provided; its data will not be sent to the gpu",
                    array = get_client_array_name(i),
                );
                continue;
            }

            if let Some(vc) = &vertex_count {
                if array.vertex_count != *vc {
                    tracing::warn!(
                        what = "found client array length mismatch",
                        array = get_client_array_name(i),
                        array_length = array.vertex_count,
                        expected_length = *vc,
                        operation = if array.vertex_count < *vc {
                            "pruned the other arrays"
                        } else {
                            "pruned this array"
                        }
                    );
                    if array.vertex_count < *vc {
                        vertex_count = Some(array.vertex_count);
                    }
                }
            } else {
                vertex_count = Some(array.vertex_count);
            }

            if array_type == PointerArrayType::TexCoord {
                if !self.is_enabled(gl_constants::GL_TEXTURE_2D) {
                    tracing::info!(
                        what = "will not assemble texcoords because GL_TEXTURE_2D is disabled"
                    );
                    continue;
                }

                if self.get_active_texture().is_none() {
                    tracing::info!(
                        what = "will not assemble texcoords because there is no active texture"
                    );
                    continue;
                }

                if !matches!(array.data_type, GLDataType::F32 | GLDataType::F64)
                    || array.element_count != 2
                {
                    tracing::info!(
                        what = "will not assemble texcoords because some crazy bastard tried to give us non-(d)vec2 texcoords"
                    );
                    continue;
                }
            }

            let size = array.data_type.size();

            let field_idx = VertexInputType::from(array_type).to_usize().unwrap();

            desc.fields[field_idx] = Some(VertexInputSpec {
                offset: desc.stride,
                data_type: array.data_type,
                num_elements: array.element_count,
            });

            layout.push(VertexBufferSlot {
                array,
                buffer_offset: desc.stride,
                data_type: array.data_type,
                array_type,
            });

            desc.stride += size * array.element_count;
            desc.align_to(4);

            if array_type == PointerArrayType::TexCoord {
                let data_type = GLDataType::U16;
                let size = data_type.size();
                let num_elements = 1;

                let field_idx = VertexInputType::TexIndex.to_usize().unwrap();

                desc.fields[field_idx] = Some(VertexInputSpec {
                    offset: desc.stride,
                    data_type,
                    num_elements,
                });

                layout.push(VertexBufferSlot {
                    array,
                    buffer_offset: desc.stride,
                    data_type,
                    array_type,
                });

                desc.stride += size * num_elements;
                desc.align_to(4);
            }
        }

        (desc, layout, vertex_count.unwrap() as usize)
    }

    fn assemble_buffer(&self) -> (VertexBufferLayout, Vec<u8>) {
        let (desc, layout, vertex_count) = self.get_vertex_buffer_layout();

        let mut buffer = Vec::new();
        buffer.resize(vertex_count * (desc.stride as usize), 0);

        for slot in &layout {
            let input_type = VertexInputType::from(slot.array_type);

            if input_type == VertexInputType::TexIndex {
                continue;
            }

            if input_type == VertexInputType::TexCoord {
                let texcoord = slot;
                let texindex = layout
                    .iter()
                    .find(|l| VertexInputType::from(l.array_type) == VertexInputType::TexIndex)
                    .unwrap();

                let bound_texture = self.get_active_texture().unwrap();

                let dest_coord_byte_size = (texcoord.data_type.size() * 2) as usize;
                let dest_index_byte_size = (texindex.data_type.size() * 2) as usize;
                let src_byte_size = (texcoord.array.data_type.size() * 2) as usize;

                for vertex_idx in 0..vertex_count {
                    let vertex_start = vertex_idx * (desc.stride as usize);
                    let dest_coord_start = vertex_start + texcoord.buffer_offset as usize;
                    let dest_index_start = vertex_start + texindex.buffer_offset as usize;
                    let src_start = vertex_idx * src_byte_size as usize;

                    let uv = match texcoord.array.data_type {
                        GLDataType::F32 => {
                            let src = &texcoord.array.data.as_ref().unwrap()
                                [src_start..src_start + src_byte_size];

                            let src = unsafe { src.align_to::<f32>().1 };

                            [src[0], src[1]]
                        }
                        GLDataType::F64 => {
                            let src = &texcoord.array.data.as_ref().unwrap()
                                [src_start..src_start + src_byte_size];

                            let src = unsafe { src.align_to::<f64>().1 };

                            [src[0] as f32, src[1] as f32]
                        }
                        _ => panic!(),
                    };

                    self.texture_lookup.
                }
            } else {
                let array = &slot.array;
                let dest_byte_size = (array.element_count * slot.data_type.size()) as usize;
                let src_byte_size = (array.element_count * array.data_type.size()) as usize;

                for vertex_idx in 0..vertex_count {
                    let dest_start =
                        vertex_idx * (desc.stride as usize) + (slot.buffer_offset as usize);
                    let src_start = vertex_idx * (src_byte_size as usize);

                    let dest = &mut buffer[dest_start..dest_start + dest_byte_size];
                    let src = &array.data.as_ref().unwrap()[src_start..src_start + src_byte_size];

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

                    if slot.array_type == PointerArrayType::Color
                        || slot.array_type == PointerArrayType::SecondaryColor
                    {
                        match slot.array.data_type {
                            GLDataType::U8 => convert_norm!(u8),
                            GLDataType::I8 => convert_norm!(i8),
                            GLDataType::U16 => convert_norm!(u16),
                            GLDataType::I16 => convert_norm!(i16),
                            GLDataType::U32 => convert_norm!(u32),
                            GLDataType::I32 => convert_norm!(i32),
                            GLDataType::F32 | GLDataType::F64 => {
                                dest.copy_from_slice(src);
                            }
                        }
                    } else {
                        match slot.array.data_type {
                            GLDataType::U8 => convert!(u8),
                            GLDataType::I8 => convert!(i8),
                            GLDataType::U16 => convert!(u16),
                            GLDataType::I16 => convert!(i16),
                            GLDataType::U32 => convert!(u32),
                            GLDataType::I32 => convert!(i32),
                            GLDataType::F32 | GLDataType::F64 => {
                                dest.copy_from_slice(src);
                            }
                        }
                    }
                }
            }
        }

        (desc, buffer)
    }

    pub fn draw_arrays(&mut self, mode: DrawMode, first: u32, count: u32) {
        if !self.client_arrays[VERTEX_ARRAY_IDX].enabled {
            tracing::warn!(
                what = "tried to call draw_arrays() without the position array set; this is invalid and the call will be ignored"
            );
            return;
        }

        let (desc, buffer) = self.assemble_buffer();

        let color = if self
            .active_flags
            .contains(&(gl_constants::GL_TEXTURE_2D as usize))
        {
            if let Some(texture) = self.get_active_texture() {
                if desc.texcoord().is_some() {
                    ColorMode::Texture { set: 1, binding: 0 }
                } else {
                    tracing::warn!(what = "GL_TEXTURE_2D was enabled but the texcoord client array wasn't enabled/valid");
                    ColorMode::Flat(DataSource::PushConstant)
                }
            } else {
                tracing::warn!(what = "GL_TEXTURE_2D was enabled but the active texture unit didn't have a bound texture", texture_unit = self.active_unit);
                ColorMode::Flat(DataSource::PushConstant)
            }
        } else {
            ColorMode::Flat(DataSource::PushConstant)
        };

        let pipeline = DynamicPipelineSpec {
            draw_mode: mode,
            vertex_buffer: desc,
            matrix: ShaderMatrixMode::MVP(DataSource::PushConstant),
            color,
            rasterization: DynamicPipelineRasterization::default(),
        };

        let push_constants = DynamicPipelinePushConstants {
            mvp: Some(self.get_mvp_matrix()),
            color: if pipeline.color == ColorMode::Flat(DataSource::PushConstant) {
                Some(self.active_color.clone().into())
            } else {
                None
            },
        };

        self.commands
            .push(RenderCommand::BindDynamicGraphicsPipeline {
                pipeline,
                push_constants,
            })
            .unwrap();

        self.commands
            .push(RenderCommand::Draw {
                start_vertex: first,
                vertex_count: count,
                data: Arc::new(buffer),
            })
            .unwrap();
    }

    pub fn get_active_texture(&self) -> Option<i32> {
        self.texture_units[self.active_unit].bound_texture.clone()
    }
}
