use std::{
    cell::RefCell,
    ops::Deref,
    sync::{Arc, Mutex},
};

use bytemuck::{Pod, Zeroable};
use enum_primitive::*;
use nalgebra_glm::{TVec3, TVec4, Vec3};
use num_derive::{FromPrimitive, ToPrimitive};

use super::{spinlock::SpinLock, textures::textures::TextureHandle};

pub static RENDER_SANDBOX_LIST: Mutex<Vec<Arc<SpinLock<Option<Vec<RenderInstruction>>>>>> =
    Mutex::new(Vec::new());

thread_local! {
    pub static RENDER_SANDBOX: Arc<SpinLock<Option<Vec<RenderInstruction>>>> = Arc::new(SpinLock::new(None));
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, PartialEq)]
#[repr(C)]
pub struct PlainVertex {
    pub position: [f32; 3],
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, PartialEq)]
#[repr(C)]
pub struct UVVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, PartialEq)]
#[repr(C)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, PartialEq)]
#[repr(C)]
pub struct UVColorVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive)]
pub enum MatrixMode {
    ModelView = gl_constants::GL_MODELVIEW,
    Projection = gl_constants::GL_PROJECTION,
    Texture = gl_constants::GL_TEXTURE,
    Color = gl_constants::GL_COLOR,
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive)]
pub enum PointerArrayType {
    Color = gl_constants::GL_COLOR_ARRAY,
    EdgeFlag = gl_constants::GL_EDGE_FLAG_ARRAY,
    FogCoord = gl_constants::GL_FOG_COORDINATE_ARRAY,
    ColorIndex = gl_constants::GL_INDEX_ARRAY,
    Normal = gl_constants::GL_NORMAL_ARRAY,
    SecondaryColor = gl_constants::GL_SECONDARY_COLOR_ARRAY,
    TexCoord = gl_constants::GL_TEXTURE_COORD_ARRAY,
    Vertex = gl_constants::GL_VERTEX_ARRAY,
}

impl PointerArrayType {
    pub fn is_supported(&self) -> bool {
        match self {
            PointerArrayType::Color => true,
            PointerArrayType::Normal => true,
            PointerArrayType::TexCoord => true,
            PointerArrayType::Vertex => true,
            PointerArrayType::EdgeFlag => false,
            PointerArrayType::FogCoord => false,
            PointerArrayType::ColorIndex => false,
            PointerArrayType::SecondaryColor => false,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive)]
pub enum PointerDataType {
    U8 = gl_constants::GL_UNSIGNED_BYTE,
    I8 = gl_constants::GL_BYTE,
    U16 = gl_constants::GL_UNSIGNED_SHORT,
    I16 = gl_constants::GL_SHORT,
    U32 = gl_constants::GL_UNSIGNED_INT,
    I32 = gl_constants::GL_INT,
    F32 = gl_constants::GL_FLOAT,
    F64 = gl_constants::GL_DOUBLE,
}

impl PointerDataType {
    pub fn size(&self) -> usize {
        match self {
            PointerDataType::U8 => 1,
            PointerDataType::I8 => 1,
            PointerDataType::U16 => 2,
            PointerDataType::I16 => 2,
            PointerDataType::U32 => 4,
            PointerDataType::I32 => 4,
            PointerDataType::F32 => 4,
            PointerDataType::F64 => 8,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive)]
pub enum DrawMode {
    Points,
    LineStrip,
    LineLoop,
    Lines,
    LineStripAdj,
    LinesAdj,
    TriStrip,
    TriFan,
    Tri,
    TriStripAdj,
    TriAdj,
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, PartialEq)]]
    pub enum RenderInstruction {
        MatrixMode(MatrixMode),
        PushMatrix,
        PopMatrix,
        LoadIdentity,
        Ortho {
            data: Box<pub struct OrthoData {
                pub left: f32,
                pub right: f32,
                pub bottom: f32,
                pub top: f32,
                pub z_near: f32,
                pub z_far: f32
            }>
        },
        Translate {
            delta: Vec3,
        },
        Rotate {
            angle: f32,
            axis: Vec3,
        },
        Scale {
            scale: Vec3,
        },
        Enable(i32),
        Disable(i32),
        Bind2DTexture(TextureHandle),
        SetColor {
            /// RGBA
            color: [f32; 4]
        },
        PlainQuads(Vec<PlainVertex>),
        UVQuads(Vec<PlainVertex>),
        PlainTris(Vec<PlainVertex>),
        UVTris(Vec<PlainVertex>),
        AlphaFunc,// TODO: this
        SetClientState {
            enabled: bool,
            array_type: PointerArrayType,
        },
        SetPointer {
            vec_count: u32,
            array_type: PointerArrayType,
            item_type: PointerDataType,
            data: Arc<Vec<u8>>,
            size: u8,
        },
        DrawArrays {
            mode: DrawMode,
            first: u32,
            count: u32,
        }
    }
}
