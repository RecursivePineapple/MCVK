use super::jni_prelude::*;

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glEnable(_: JNIEnv<'_>, _: JClass<'_>, cap: jint) {
    push_instruction(RenderInstruction::Enable(cap));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glDisable(_: JNIEnv<'_>, _: JClass<'_>, cap: jint) {
    push_instruction(RenderInstruction::Disable(cap));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
unsafe fn glClear(_: JNIEnv<'_>, _: JClass<'_>, mask: jint) {
    let mask = mask as u32;

    let depth = (mask & GL_DEPTH_BUFFER_BIT) == GL_DEPTH_BUFFER_BIT;
    let colour = (mask & GL_COLOR_BUFFER_BIT) == GL_COLOR_BUFFER_BIT;

    if colour {
        write_instance_into!(inst);

        inst.on_clear_colour();
    } else {
        if depth {
            push_instruction(RenderInstruction::ClearDepth);
        }
    }
}
