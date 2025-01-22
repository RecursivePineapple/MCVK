use super::jni_prelude::*;

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glMatrixMode(_: JNIEnv<'_>, _: JClass<'_>, mode: jint) {
    push_instruction(RenderInstruction::MatrixMode(
        MatrixMode::from_i32(mode).unwrap(),
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
unsafe fn glTranslated(_: JNIEnv<'_>, _: JClass<'_>, x: jdouble, y: jdouble, z: jdouble) {
    push_instruction(RenderInstruction::Translate {
        delta: Vec3::new(x as f32, y as f32, z as f32),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glScalef(_: JNIEnv<'_>, _: JClass<'_>, x: jfloat, y: jfloat, z: jfloat) {
    push_instruction(RenderInstruction::Scale {
        scale: Vec3::new(x, y, z),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glScaled(_: JNIEnv<'_>, _: JClass<'_>, x: jdouble, y: jdouble, z: jdouble) {
    push_instruction(RenderInstruction::Scale {
        scale: Vec3::new(x as f32, y as f32, z as f32),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glRotatef(_: JNIEnv<'_>, _: JClass<'_>, angle: jfloat, x: jfloat, y: jfloat, z: jfloat) {
    push_instruction(RenderInstruction::Rotate {
        angle,
        axis: Vec3::new(x, y, z),
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glRotated(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    angle: jdouble,
    x: jdouble,
    y: jdouble,
    z: jdouble,
) {
    push_instruction(RenderInstruction::Rotate {
        angle: angle as f32,
        axis: Vec3::new(x as f32, y as f32, z as f32),
    });
}
