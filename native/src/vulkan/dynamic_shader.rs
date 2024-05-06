use std::{
    hash::Hash,
    mem::{discriminant, transmute},
};

use nalgebra_glm::{TMat4, Vec4};
use vulkano::format::Format;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct VertexInputSpec {
    pub name: String,
    pub format: Format,
    pub num_elements: u32,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ShaderMatrixMode {
    /// P * V * M in a mat4 provided via push constants
    MVP_PC,
    /// P * V, M in a mat4 provided via push constants
    M_PC_VP_PC,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    Flat,
    Texture(Option<VertexInputSpec>),
    Array(VertexInputSpec),
}

impl Hash for ColorMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
        match self {
            ColorMode::Flat => {}
            ColorMode::Texture(texture) => texture.hash(state),
            ColorMode::Array(array) => array.hash(state),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DynamicPipelinePushConstants {
    // MVPSeparate { model: TMat4<f32>, vp: TMat4<f32> },
    MVP(TMat4<f32>),
    Color(Vec4),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct DynamicPipelineSpec {
    pub position: VertexInputSpec,
    pub normal: Option<VertexInputSpec>,
    pub color: ColorMode,
    pub matrix: ShaderMatrixMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShaderDataTypeOrdinal {
    UInt,
    Int,
    Float,
    Double,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShaderDataType {
    UInt(u32),
    Int(u32),
    Float(u32),
    Double(u32),
}

impl ShaderDataType {
    pub fn from_vertex_input(input: &VertexInputSpec) -> Option<Self> {
        Self::from_format_and_size(&input.format, input.num_elements)
    }

    pub fn from_format_and_size(format: &Format, size: u32) -> Option<Self> {
        if size > 4 {
            return None;
        }

        match format {
            Format::R8_UINT | Format::R16_UINT | Format::R32_UINT => Some(Self::UInt(size)),
            Format::R8_SINT | Format::R16_SINT | Format::R32_SINT => Some(Self::Int(size)),
            Format::R32_SFLOAT => Some(Self::Float(size)),
            Format::R64_SFLOAT => Some(Self::Double(size)),
            _ => None,
        }
    }

    pub fn ordinal(&self) -> ShaderDataTypeOrdinal {
        match self {
            ShaderDataType::UInt(_) => ShaderDataTypeOrdinal::UInt,
            ShaderDataType::Int(_) => ShaderDataTypeOrdinal::Int,
            ShaderDataType::Float(_) => ShaderDataTypeOrdinal::Float,
            ShaderDataType::Double(_) => ShaderDataTypeOrdinal::Double,
        }
    }

    pub fn size(&self) -> u32 {
        match self {
            ShaderDataType::UInt(size)
            | ShaderDataType::Int(size)
            | ShaderDataType::Float(size)
            | ShaderDataType::Double(size) => *size,
        }
    }

    pub fn as_strs(&self) -> (&'static str, &'static str) {
        let size = self.size();

        if size <= 1 {
            match self {
                Self::UInt(_) => ("uint", ""),
                Self::Int(_) => ("int", ""),
                Self::Float(_) => ("float", ""),
                Self::Double(_) => ("double", ""),
            }
        } else {
            let suffix = match size {
                2 => "2",
                3 => "3",
                4 => "4",
                _ => panic!("illegal shader input size {size}"),
            };

            match self {
                Self::UInt(_) => ("uvec", suffix),
                Self::Int(_) => ("ivec", suffix),
                Self::Float(_) => ("vec", suffix),
                Self::Double(_) => ("dvec", suffix),
            }
        }
    }
}

fn get_widening_zeroes(size: u32) -> &'static str {
    match size {
        1 => ", 0.0, 0.0, 0.0",
        2 => ", 0.0, 0.0",
        3 => ", 0.0",
        _ => "",
    }
}

fn get_widening_ones(size: u32) -> &'static str {
    match size {
        1 => ", 1.0, 1.0, 1.0",
        2 => ", 1.0, 1.0",
        3 => ", 1.0",
        _ => "",
    }
}

impl DynamicPipelineSpec {
    fn append_io(
        code: &mut String,
        location: u32,
        is_input: bool,
        var_type: &ShaderDataType,
        name: &str,
    ) {
        let (tprefix, tsuffix) = var_type.as_strs();
        *code += &concat_string::concat_string!(
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

    pub fn get_vertex_shader_code(&self) -> String {
        let mut code = String::with_capacity(1024);

        code += "#version 450\n";

        Self::append_io(
            &mut code,
            0,
            true,
            &ShaderDataType::from_vertex_input(&self.position).unwrap(),
            &self.position.name,
        );

        if let Some(normal) = self.normal.as_ref() {
            Self::append_io(
                &mut code,
                1,
                true,
                &ShaderDataType::from_vertex_input(&normal).unwrap(),
                &normal.name,
            );
        }

        match &self.color {
            ColorMode::Flat => {}
            ColorMode::Texture(Some(texcoord)) => {
                Self::append_io(
                    &mut code,
                    2,
                    true,
                    &ShaderDataType::from_vertex_input(&texcoord).unwrap(),
                    &texcoord.name,
                );
            }
            ColorMode::Texture(None) => {}
            ColorMode::Array(colors) => {
                Self::append_io(
                    &mut code,
                    2,
                    true,
                    &ShaderDataType::from_vertex_input(&colors).unwrap(),
                    &colors.name,
                );
            }
        }

        code += "layout(push_constant) uniform constants {\n";

        match &self.matrix {
            ShaderMatrixMode::MVP_PC => {
                code += "  mat4 mvp\n";
            }
            ShaderMatrixMode::M_PC_VP_PC => {
                code += "  mat4 model;\n";
                code += "  mat4 vp;\n";
            }
        }

        if let ColorMode::Flat = &self.color {
            code += "  vec4 color;\n";
        }

        code += "} PushConstants;\n";

        match &self.color {
            ColorMode::Flat | ColorMode::Array(_) => {
                Self::append_io(&mut code, 0, false, &ShaderDataType::Float(4), "frag_color");
            }
            ColorMode::Texture(Some(_)) => {
                Self::append_io(&mut code, 0, false, &ShaderDataType::Float(2), "tex_coord");
            }
            _ => {}
        }

        if self.normal.is_some() {
            Self::append_io(
                &mut code,
                1,
                false,
                &ShaderDataType::Float(3),
                "frag_normal",
            );
        }

        code += "void main() {\n";

        match &self.matrix {
            ShaderMatrixMode::MVP_PC => {
                code += &concat_string::concat_string!(
                    "  gl_Position = vec4(PushConstants.mvp * ",
                    &self.position.name,
                    get_widening_zeroes(self.position.num_elements),
                    ");\n"
                );
            }
            ShaderMatrixMode::M_PC_VP_PC => {
                code += &concat_string::concat_string!(
                    "  gl_Position = vec4(PushConstants.model * PushConstants.vp * ",
                    &self.position.name,
                    get_widening_zeroes(self.position.num_elements),
                    ");\n"
                );
            }
        }

        code += "}\n";

        code
    }

    pub fn get_fragment_shader_code(&self) -> String {
        let mut code = String::new();

        code += "#version 450\n";

        code
    }
}
