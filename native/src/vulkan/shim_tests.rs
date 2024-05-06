use std::{mem::MaybeUninit, sync::Arc};

use jni::{objects::JClass, JNIEnv};
use num::ToPrimitive;
use vulkano::format::Format;
use vulkano::pipeline::graphics::vertex_input::{
    VertexBufferDescription, VertexInputRate, VertexMemberInfo,
};

use crate::vulkan::sandbox::{RenderInstruction, RENDER_SANDBOX};

use super::dynamic_shader;
use super::{
    insn_assembler::RenderInsnAssembler,
    sandbox::{PointerArrayType, PointerDataType},
    sandbox_jni,
};

unsafe fn env() -> JNIEnv<'static> {
    MaybeUninit::<JNIEnv<'static>>::uninit().assume_init()
}

unsafe fn class() -> JClass<'static> {
    MaybeUninit::<JClass<'static>>::uninit().assume_init()
}

#[test]
fn add_pointer_u8_packed() {
    unsafe {
        let mut data = Vec::<u8>::new();
        data.resize(15, 0);

        for i in 0..15 {
            data[i] = i as u8;
        }

        RENDER_SANDBOX.with(|l| {
            l.lock().take();
        });

        sandbox_jni::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            0,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::U8.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        RENDER_SANDBOX.with(|l| {
            let insn1 = RenderInstruction::SetPointer {
                vec_count: 5,
                array_type: PointerArrayType::Color,
                item_type: PointerDataType::U8,
                data: Arc::new(data.clone()),
                size: 3,
            };

            let g = l.lock();

            dbg!(&*g);
            assert!(*g == Some(vec![insn1]));
        });
    }
}

#[test]
fn add_pointer_u8() {
    unsafe {
        let mut data = Vec::<u8>::new();
        data.resize(20, 0);

        let mut data_compact = Vec::<u8>::new();
        data_compact.resize(15, 0);

        for i in 0..15 {
            data[i + i / 3] = i as u8;
            data_compact[i] = i as u8;
        }

        RENDER_SANDBOX.with(|l| {
            l.lock().take();
        });

        sandbox_jni::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            4,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::U8.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        RENDER_SANDBOX.with(|l| {
            let insn1 = RenderInstruction::SetPointer {
                vec_count: 5,
                array_type: PointerArrayType::Color,
                item_type: PointerDataType::U8,
                data: Arc::new(data_compact.clone()),
                size: 3,
            };

            let g = l.lock();

            dbg!(&*g);
            assert!(*g == Some(vec![insn1]));
        });
    }
}

#[test]
fn add_pointer_f32() {
    unsafe {
        let mut data = Vec::<u8>::new();
        data.resize(4 * 20, 0);

        let (_, data2, _) = data.align_to_mut::<f32>();

        let mut data_compact = Vec::<u8>::new();
        data_compact.resize(4 * 15, 0);

        let (_, data_compact2, _) = data_compact.align_to_mut::<f32>();

        for i in 0..15 {
            data2[i + i / 3] = i as f32;
            data_compact2[i] = i as f32;
        }

        RENDER_SANDBOX.with(|l| {
            l.lock().take();
        });

        sandbox_jni::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            16,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::F32.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        RENDER_SANDBOX.with(|l| {
            let insn1 = RenderInstruction::SetPointer {
                vec_count: 5,
                array_type: PointerArrayType::Color,
                item_type: PointerDataType::F32,
                data: Arc::new(data_compact.clone()),
                size: 3,
            };

            let g = l.lock();

            dbg!(&*g);
            assert!(*g == Some(vec![insn1]));
        });
    }
}

#[test]
fn vertex_assembly() {
    let mut asm = RenderInsnAssembler::new();

    let mut pos = (0..3 * 10).map(|i| i as f32).collect::<Vec<_>>();
    let mut color = (0..3 * 10).map(|i| i as f32).rev().collect::<Vec<_>>();

    asm.assemble(&[
        RenderInstruction::SetClientState {
            enabled: true,
            array_type: PointerArrayType::Vertex,
        },
        RenderInstruction::SetClientState {
            enabled: true,
            array_type: PointerArrayType::Color,
        },
        RenderInstruction::SetPointer {
            vec_count: 10,
            array_type: PointerArrayType::Vertex,
            item_type: PointerDataType::F32,
            data: Arc::new(unsafe { pos.align_to().1.to_owned() }),
            size: 3,
        },
        RenderInstruction::SetPointer {
            vec_count: 10,
            array_type: PointerArrayType::Color,
            item_type: PointerDataType::F32,
            data: Arc::new(unsafe { color.align_to().1.to_owned() }),
            size: 3,
        },
    ]);

    let (pipeline, push_constants, desc, buffer) = asm.assemble_vertices().unwrap();

    match desc {
        VertexBufferDescription {
            members,
            stride: 24,
            input_rate: VertexInputRate::Vertex,
        } => {
            match members.get("pos").unwrap() {
                VertexMemberInfo {
                    format: Format::R32_SFLOAT,
                    num_elements: 3,
                    offset: 12,
                } => {}
                _ => panic!("bad pos VertexMemberInfo"),
            }
            match members.get("color").unwrap() {
                VertexMemberInfo {
                    format: Format::R32_SFLOAT,
                    num_elements: 3,
                    offset: 0,
                } => {}
                _ => panic!("bad color VertexMemberInfo"),
            }
        }
        _ => panic!("bad VertexBufferDescription"),
    }

    let mut target = Vec::new();

    for i in 0..10 {
        target.extend_from_slice(&color[i * 3..(i + 1) * 3]);
        target.extend_from_slice(&pos[i * 3..(i + 1) * 3]);
    }

    let target = unsafe { target.align_to::<u8>().1.to_owned() };

    assert_eq!(buffer, target);
}

#[test]
fn shader_test() {
    let pipeline = dynamic_shader::DynamicPipelineSpec {
        color: dynamic_shader::ColorMode::Flat,
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
