use native_macros::gl_fn_decl;

use super::jni_prelude::*;

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glBegin(mut env: JNIEnv<'_>, _: JClass<'_>, mode: jint) {
    if let Some(mode) = DrawMode::from_i32(mode) {
        push_instruction(RenderInstruction::Begin(mode));
    } else {
        tracing::warn!(
            what = "glBegin was called with an invalid parameter and the call has been ignored!",
            mode
        );
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glEnd(mut env: JNIEnv<'_>, _: JClass<'_>) {
    push_instruction(RenderInstruction::End);
}

gl_fn_decl!(
    "com.recursive_pineapple.mcvk.rendering.RenderSandboxGen",
    glVertex,
    [2, 3, 4],
    [0.0, 0.0, 0.0, 1.0],
    |x, y, z, w| RenderInstruction::Vertex([x, y, z, w].into())
);

gl_fn_decl!(
    "com.recursive_pineapple.mcvk.rendering.RenderSandboxGen",
    glTexCoord,
    [2, 3, 4],
    [0.0, 0.0, 0.0, 1.0],
    |x, y, z, w| RenderInstruction::TexCoord([x, y, z, w].into())
);

gl_fn_decl!(
    "com.recursive_pineapple.mcvk.rendering.RenderSandboxGen",
    glNormal,
    [2, 3, 4],
    [0.0, 0.0, 0.0, 1.0],
    |x, y, z, w| RenderInstruction::TexCoord([x, y, z, w].into())
);

gl_fn_decl!(
    "com.recursive_pineapple.mcvk.rendering.RenderSandboxGen",
    glColor,
    [3, 4],
    [0.0, 0.0, 0.0, 1.0],
    |x, y, z, w| RenderInstruction::SetColor([x, y, z, w].into())
);
