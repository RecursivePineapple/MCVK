use crate::vulkan::dynamic_shader::*;

#[test]
fn shader_test() {
    let shader_spec = ShaderSpec {
        color: ColorMode::Texture { set: 1, binding: 0 },
        matrix: ShaderMatrixMode::MVP(DataSource::PushConstant),
        vertex_buffer: VertexBufferLayout {
            fields: [
                Some(VertexInputSpec {
                    data_type: VertexDataType::F32,
                    num_elements: 3,
                    offset: 0,
                }),
                Some(VertexInputSpec {
                    data_type: VertexDataType::F32,
                    num_elements: 3,
                    offset: 12,
                }),
                None,
                Some(VertexInputSpec {
                    data_type: VertexDataType::F32,
                    num_elements: 2,
                    offset: 12 + 12,
                }),
            ],
            stride: 12 + 12 + 8,
        },
    };

    println!("{}", &shader_spec.get_vertex_shader_code());
    println!("{}", &shader_spec.get_fragment_shader_code());

    panic!();
}
