use std::{mem::MaybeUninit, sync::Arc};

use jni::{objects::JClass, JNIEnv};
use num::ToPrimitive;

use crate::vulkan::sandbox::{RenderInstruction, RENDER_SANDBOX};
use crate::vulkan::sandbox_jni::jni_prelude::{DrawMode, RenderSandbox};

use super::commands::CommandQueue;
use super::dynamic_shader;
use super::sandbox::{put_sandbox, take_sandbox};
use super::sandbox_jni::client_arrays;
use super::{
    insn_assembler::RenderInsnAssembler,
    sandbox::{PointerArrayType, PointerDataType},
};

unsafe fn env() -> JNIEnv<'static> {
    #[allow(invalid_value)]
    MaybeUninit::<JNIEnv<'static>>::uninit().assume_init()
}

unsafe fn class() -> JClass<'static> {
    #[allow(invalid_value)]
    MaybeUninit::<JClass<'static>>::uninit().assume_init()
}

fn prepare_sandbox() {
    take_sandbox();
    put_sandbox(RenderSandbox::List(Vec::new()));
}

fn assert_insns(v: &Vec<RenderInstruction>) {
    RENDER_SANDBOX.with(|l| {
        let g = l.lock();

        dbg!(&*g);

        match &*g {
            RenderSandbox::List(insns) => {
                assert!(insns == v);
            }
            _ => panic!(),
        }
    });
}

#[test]
fn add_pointer_u8_packed() {
    unsafe {
        let mut data = Vec::<u8>::new();
        data.resize(15, 0);

        for i in 0..15 {
            data[i] = i as u8;
        }

        prepare_sandbox();

        client_arrays::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            0,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::U8.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        assert_insns(&vec![RenderInstruction::SetPointer {
            vec_count: 5,
            array_type: PointerArrayType::Color,
            item_type: PointerDataType::U8,
            data: Arc::new(data.clone()),
            size: 3,
        }]);
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

        prepare_sandbox();

        client_arrays::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            4,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::U8.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        assert_insns(&vec![RenderInstruction::SetPointer {
            vec_count: 5,
            array_type: PointerArrayType::Color,
            item_type: PointerDataType::U8,
            data: Arc::new(data_compact.clone()),
            size: 3,
        }]);
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

        prepare_sandbox();

        client_arrays::Java_com_recursive_1pineapple_mcvk_rendering_RenderSandbox_addPointerArray(
            env(),
            class(),
            3,
            16,
            PointerArrayType::Color.to_i32().unwrap(),
            PointerDataType::F32.to_i32().unwrap(),
            data.as_ptr(),
            data.len() as i32,
        );

        assert_insns(&vec![RenderInstruction::SetPointer {
            vec_count: 5,
            array_type: PointerArrayType::Color,
            item_type: PointerDataType::F32,
            data: Arc::new(data_compact.clone()),
            size: 3,
        }]);
    }
}

#[test]
fn vertex_assembly() {
    let mut asm = RenderInsnAssembler::new(CommandQueue::Buffered(Vec::new()));

    let pos = (0..3 * 10).map(|i| i as f32).collect::<Vec<_>>();
    let color = (0..3 * 10).map(|i| i as f32).rev().collect::<Vec<_>>();

    asm.feed(&[
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

    asm.draw_arrays(DrawMode::Tri, 0, 3);

    let mut target = Vec::new();

    for i in 0..10 {
        target.extend_from_slice(&color[i * 3..(i + 1) * 3]);
        target.extend_from_slice(&pos[i * 3..(i + 1) * 3]);
    }

    let target = unsafe { target.align_to::<u8>().1.to_owned() };

    // assert_eq!(
    //     &asm.commands[..],
    //     &[
    //         RenderCommand::BindDynamicGraphicsPipeline {
    //             pipeline: DynamicPipelineSpec {
    //                 position: todo!(),
    //                 normal: todo!(),
    //                 color: todo!(),
    //                 matrix: todo!()
    //             },
    //             push_constants: vec![]
    //         },
    //         RenderCommand::Draw {
    //             mode: (),
    //             vertex: (),
    //             start_vertex: (),
    //             vertex_count: (),
    //             data: ()
    //         }
    //     ]
    // );

    // match desc {
    //     VertexBufferDescription {
    //         members,
    //         stride: 24,
    //         input_rate: VertexInputRate::Vertex,
    //     } => {
    //         match members.get("pos").unwrap() {
    //             VertexMemberInfo {
    //                 format: Format::R32_SFLOAT,
    //                 num_elements: 3,
    //                 offset: 12,
    //             } => {}
    //             _ => panic!("bad pos VertexMemberInfo"),
    //         }
    //         match members.get("color").unwrap() {
    //             VertexMemberInfo {
    //                 format: Format::R32_SFLOAT,
    //                 num_elements: 3,
    //                 offset: 0,
    //             } => {}
    //             _ => panic!("bad color VertexMemberInfo"),
    //         }
    //     }
    //     _ => panic!("bad VertexBufferDescription"),
    // }

    // assert_eq!(buffer, target);
}
