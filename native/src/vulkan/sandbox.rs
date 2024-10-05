use std::sync::{atomic::AtomicI32, Arc, Mutex};

use nalgebra_glm::{DVec3, Vec3, Vec4};
use num_derive::{FromPrimitive, ToPrimitive};

use super::{insn_assembler::RenderInsnAssembler, spinlock::SpinLock};

pub type RenderSandboxStack = Arc<SpinLock<RenderSandbox>>;

thread_local! {
    pub static RENDER_SANDBOX: RenderSandboxStack = Arc::new(SpinLock::new(RenderSandbox::None));
}

#[derive(Debug)]
pub enum RenderSandbox {
    Assembler(Box<RenderInsnAssembler>),
    List(Vec<RenderInstruction>),
    None,
}

impl RenderSandbox {
    pub fn push(&mut self, insn: RenderInstruction) {
        match self {
            Self::Assembler(asm) => asm.feed(&[insn]),
            Self::List(insns) => insns.push(insn),
            Self::None => {
                tracing::error!(
                    what = "tried to push render instruction on invalid thread",
                    ?insn,
                    thread = std::thread::current().name()
                );
            }
        }
    }

    pub fn get_bound_texture(&self) -> Option<i32> {
        match self {
            Self::Assembler(a) => a.get_active_texture(),
            Self::List(l) => {
                for insn in l.iter().rev() {
                    if let RenderInstruction::BindTexture(i) = insn {
                        return Some(*i);
                    }
                }

                None
            }
            Self::None => None,
        }
    }
}

pub fn push_instruction(insn: RenderInstruction) {
    RENDER_SANDBOX.with(|lock| {
        let mut guard = lock.lock();

        guard.push(insn);
    });
}

pub fn with_render_sandbox<F: FnOnce(&mut RenderSandbox) -> R, R>(f: F) -> R {
    RENDER_SANDBOX.with(|lock| {
        let mut guard = lock.lock();

        f(&mut guard)
    })
}

pub fn put_sandbox(sandbox: RenderSandbox) {
    RENDER_SANDBOX.with(|lock| {
        let mut guard = lock.lock();

        if matches!(&*guard, RenderSandbox::None) {
            *guard = sandbox;
        } else {
            panic!("tried to put_sandbox when a sandbox was already active\n({guard:?})");
        }
    });
}

pub fn take_sandbox() -> Option<RenderSandbox> {
    let existing = RENDER_SANDBOX.with(|lock| {
        let mut guard = lock.lock();

        let mut none = RenderSandbox::None;
        std::mem::swap(&mut *guard, &mut none);
        none
    });

    if matches!(&existing, RenderSandbox::None) {
        None
    } else {
        Some(existing)
    }
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

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive)]
pub enum DrawMode {
    Points = gl_constants::GL_POINTS,
    LineStrip = gl_constants::GL_LINE_STRIP,
    LineLoop = gl_constants::GL_LINE_LOOP,
    Lines = gl_constants::GL_LINES,
    LineStripAdj = gl_constants::GL_LINE_STRIP_ADJACENCY,
    LinesAdj = gl_constants::GL_LINES_ADJACENCY,
    TriStrip = gl_constants::GL_TRIANGLE_STRIP,
    TriFan = gl_constants::GL_TRIANGLE_FAN,
    Tri = gl_constants::GL_TRIANGLES,
    TriStripAdj = gl_constants::GL_TRIANGLE_STRIP_ADJACENCY,
    TriAdj = gl_constants::GL_TRIANGLES_ADJACENCY,
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
        Translated {
            delta: DVec3,
        },
        Rotate {
            angle: f32,
            axis: Vec3,
        },
        Rotated {
            angle: f64,
            axis: DVec3,
        },
        Scale {
            scale: Vec3,
        },
        Scaled {
            scale: DVec3,
        },

        Enable(i32),
        Disable(i32),

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
        },

        SetActiveTextureUnit(usize),
        BindTexture(i32),

        TexCoord(Vec4),

        SetColor(Vec4),

        Begin(DrawMode),
        Vertex(Vec4),
        End,

        AlphaFunc,

        ClearDepth,
    }
}
