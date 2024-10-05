use std::mem::size_of_val;

use super::jni_prelude::*;

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
