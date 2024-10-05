use crate::vulkan::dynamic_shader;

#[test]
fn shader_test() {
    let pipeline = dynamic_shader::DynamicPipelineSpec {
        color: dynamic_shader::ColorMode::Flat_PC,
        matrix: dynamic_shader::ShaderMatrixMode::MVP_PC,
        normal: None,
        position: dynamic_shader::VertexInputSpec {
            name: "position".to_owned(),
            format: vulkano::format::Format::R8_UINT,
            num_elements: 3,
        },
    };

    println!("{}", &pipeline.get_vertex_shader_code());

    panic!();
}
