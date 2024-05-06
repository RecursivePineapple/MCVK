use std::{mem::size_of_val, sync::Arc};

use enum_primitive::FromPrimitive;
use jni::{
    objects::JClass,
    sys::{jdouble, jfloat, jint, jlong},
    JNIEnv,
};
use nalgebra_glm::Vec3;
use native_macros::jni_export;

use super::sandbox::{
    DrawMode, OrthoData, PointerArrayType, PointerDataType, RenderInstruction, RENDER_SANDBOX,
    RENDER_SANDBOX_LIST,
};

fn push_instruction(insn: RenderInstruction) {
    RENDER_SANDBOX.with(|lock| {
        let mut guard = lock.lock();

        if guard.is_none() {
            *guard = Some(Vec::with_capacity(4096));
            RENDER_SANDBOX_LIST.lock().unwrap().push(lock.clone());
        }

        guard.as_mut().unwrap().push(insn);
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glMatrixMode(_: JNIEnv<'_>, _: JClass<'_>, mode: jint) {
    push_instruction(RenderInstruction::MatrixMode(
        super::sandbox::MatrixMode::from_i32(mode).unwrap(),
    ));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glPushMatrix(_: JNIEnv<'_>, _: JClass<'_>) {
    push_instruction(RenderInstruction::PushMatrix);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glPopMatrix(_: JNIEnv<'_>, _: JClass<'_>) {
    push_instruction(RenderInstruction::PopMatrix);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glLoadIdentity(_: JNIEnv<'_>, _: JClass<'_>) {
    push_instruction(RenderInstruction::LoadIdentity);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glOrtho(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    left: jdouble,
    right: jdouble,
    bottom: jdouble,
    top: jdouble,
    z_near: jdouble,
    z_far: jdouble,
) {
    push_instruction(RenderInstruction::Ortho {
        data: Box::new(OrthoData {
            left: left as f32,
            right: right as f32,
            bottom: bottom as f32,
            top: top as f32,
            z_near: z_near as f32,
            z_far: z_far as f32,
        }),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glTranslatef(_: JNIEnv<'_>, _: JClass<'_>, x: jfloat, y: jfloat, z: jfloat) {
    push_instruction(RenderInstruction::Translate {
        delta: Vec3::new(x, y, z),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glEnable(_: JNIEnv<'_>, _: JClass<'_>, cap: jint) {
    push_instruction(RenderInstruction::Enable(cap));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glDisable(_: JNIEnv<'_>, _: JClass<'_>, cap: jint) {
    push_instruction(RenderInstruction::Disable(cap));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glEnableClientState(_: JNIEnv<'_>, _: JClass<'_>, array_type: jint) {
    let array_type = PointerArrayType::from_i32(array_type).unwrap();

    if !array_type.is_supported() {
        return;
    }

    push_instruction(RenderInstruction::SetClientState {
        enabled: true,
        array_type,
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glDisableClientState(_: JNIEnv<'_>, _: JClass<'_>, array_type: jint) {
    let array_type = PointerArrayType::from_i32(array_type).unwrap();

    if !array_type.is_supported() {
        return;
    }

    push_instruction(RenderInstruction::SetClientState {
        enabled: false,
        array_type,
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn addPointerArray(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    size: jint,
    stride: jint,
    array_type: jint,
    item_type: jint,
    start: *const u8,
    byte_length: jint,
) {
    // make sure the implicit conversion for start is sound
    assert_eq!(size_of_val(&(0 as jlong)), size_of_val(&start));

    let size = size as usize;
    let stride = stride as usize;
    let byte_length = byte_length as usize;
    let array_type = PointerArrayType::from_i32(array_type).unwrap();

    if !array_type.is_supported() {
        return;
    }

    assert!(size <= 4);

    let data = std::slice::from_raw_parts(start, byte_length);

    let item_type = PointerDataType::from_i32(item_type).unwrap();
    let item_size = item_type.size();

    let vec_byte_size = size * item_size;
    let stride = if stride > 0 { stride } else { vec_byte_size };

    let vec_count = byte_length / stride;

    let mut out = Vec::with_capacity(vec_byte_size * vec_count);
    out.resize(vec_byte_size * vec_count, 0);

    for vec_idx in 0..vec_count {
        let dest = &mut out[(vec_idx * vec_byte_size)..(vec_idx * vec_byte_size + vec_byte_size)];
        let src = &data[(vec_idx * stride)..(vec_idx * stride + vec_byte_size)];

        dest.copy_from_slice(src);
    }

    push_instruction(RenderInstruction::SetPointer {
        size: size as u8,
        vec_count: vec_count as u32,
        array_type,
        item_type,
        data: Arc::new(out),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glDrawArrays(_: JNIEnv<'_>, _: JClass<'_>, mode: jint, first: jint, count: jint) {
    push_instruction(RenderInstruction::DrawArrays {
        mode: DrawMode::from_i32(mode).unwrap(),
        first: first as u32,
        count: count as u32,
    });
}
