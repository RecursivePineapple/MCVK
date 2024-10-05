use std::mem::transmute;

use image::Rgba;
use image::RgbaImage;
use serde::Deserialize;
use serde::Serialize;

use crate::vulkan::textures::textures::AnimationMetadata;
use crate::vulkan::textures::textures::TextureImage;

use super::jni_prelude::*;

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glGenTextures(_: JNIEnv<'_>, _: JClass<'_>) -> jint {
    write_instance_into!(inst);

    let tid = inst.textures.borrow_mut().create_texture();

    transmute(tid)
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glDeleteTextures(_: JNIEnv<'_>, _: JClass<'_>, texture: jint) {
    write_instance_into!(inst);

    inst.textures.borrow_mut().free_texture(transmute(texture));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glBindTexture(_: JNIEnv<'_>, _: JClass<'_>, target: jint, texture: jint) {
    if target as u32 != GL_TEXTURE_2D {
        tracing::warn!(
            what =
                "glBindTexture() was called with target other than GL_TEXTURE_2D: this is a no-op!",
            target,
            texture
        );
        return;
    }

    push_instruction(RenderInstruction::BindTexture(texture));
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
#[allow(unused)]
pub unsafe fn glTexImage2D(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    target: jint,
    mip_level: jint,
    gpu_format: jint,
    width: jint,
    height: jint,
    border: jint,
    cpu_format: jint,
    data: JByteBuffer,
) {
    if target as u32 != GL_TEXTURE_2D {
        tracing::warn!(
            what =
                "glTexImage2D() was called with target other than GL_TEXTURE_2D: this is a no-op!",
            target
        );
        return;
    }

    jni_todo!(env);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
#[allow(unused)]
pub unsafe fn glTextureSubImage2D(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    target: jint,
    mip_level: jint,
    xoffset: jint,
    yoffset: jint,
    width: jint,
    height: jint,
    border: jint,
    cpu_format: jint,
    data: JByteBuffer,
) {
    if target as u32 != GL_TEXTURE_2D {
        tracing::warn!(
            what =
                "glTexImage2D() was called with target other than GL_TEXTURE_2D: this is a no-op!",
            target
        );
        return;
    }

    jni_todo!(env);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glTexParameterf(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    #[allow(unused)] target: jint,
    pname: jint,
    param: jfloat,
) {
    let bound_texture = with_render_sandbox(|s| s.get_bound_texture());

    let texture = bound_texture.and_then(|t| {
        read_field_into!(inst; textures);

        textures.get_texture_handle(t as u32).cloned()
    });

    match texture {
        Some(t) => {
            t.set_tex_param(pname as u32, param);
        }
        None => {
            tracing::warn!(what = "tried to call glTexParameterf with no bound texture", pname, param = ?param);
        }
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glTexParameteri(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    #[allow(unused)] target: jint,
    pname: jint,
    param: jint,
) {
    let bound_texture = with_render_sandbox(|s| s.get_bound_texture());

    let texture = bound_texture.and_then(|t| {
        write_field_into!(inst; textures);

        textures.get_texture_handle(t as u32).cloned()
    });

    match texture {
        Some(t) => {
            t.set_tex_param(pname as u32, param);
        }
        None => {
            tracing::warn!(what = "tried to call glTexParameteri with no bound texture", pname, param = ?param);
        }
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glGetTexParameterf(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    #[allow(unused)] target: jint,
    pname: jint,
    param: jfloat,
) -> jfloat {
    let bound_texture = with_render_sandbox(|s| s.get_bound_texture());

    let texture = bound_texture.and_then(|t| {
        write_field_into!(inst; textures);

        textures.get_texture_handle(t as u32).cloned()
    });

    match texture {
        Some(t) => t.get_tex_param(pname as u32),
        None => {
            tracing::warn!(what = "tried to call glGetTexParameterf with no bound texture", pname, param = ?param);
            0.0
        }
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.RenderSandbox")]
pub unsafe fn glGetTexParameteri(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    #[allow(unused)] target: jint,
    pname: jint,
    param: jint,
) -> jint {
    let bound_texture = with_render_sandbox(|s| s.get_bound_texture());

    let texture = bound_texture.and_then(|t| {
        write_field_into!(inst; textures);

        textures.get_texture_handle(t as u32).cloned()
    });

    match texture {
        Some(t) => t.get_tex_param(pname as u32),
        None => {
            tracing::warn!(what = "tried to call glGetTexParameteri with no bound texture", pname, param = ?param);
            0
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AnimationMetadataSection {
    #[serde(rename = "animationFrames")]
    animation_frames: Vec<AnimationMetadataFrame>,
    #[serde(rename = "frameWidth")]
    frame_width: i32,
    #[serde(rename = "frameHeight")]
    frame_height: i32,
    #[serde(rename = "frameTime")]
    frame_time: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AnimationMetadataFrame {
    #[serde(rename = "frameIndex")]
    frame_index: i32,
    #[serde(rename = "frameTime")]
    frame_time: i32,
}

fn get_animation_metadata(
    env: &mut JNIEnv<'_>,
    animation: JString<'_>,
) -> Option<AnimationMetadata> {
    let animation: String = env.get_string(&animation).unwrap().into();

    let section = serde_json::from_str::<Option<AnimationMetadataSection>>(&animation).unwrap()?;

    let mut frames = Vec::new();

    for frame in &section.animation_frames {
        let frame_time = if frame.frame_time == -1 {
            section.frame_time
        } else {
            frame.frame_time
        };

        frames.reserve(frame_time as usize);

        for _ in 0..frame_time {
            frames.push(frame.frame_index as u16);
        }
    }

    Some(AnimationMetadata {
        animation_frames: frames,
    })
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueMissingSprite(mut env: JNIEnv<'_>, _: JClass<'_>, name: JString<'_>) {
    write_field_into!(inst; textures);

    throw!(
        env,
        textures.enqueue_sprite(
            env.get_string_unchecked(&name).unwrap().into(),
            TextureImage::None,
        )
    );
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueFrameSprite(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    name: JString<'_>,
    width: jint,
    height: jint,
    frames: JObjectArray<'_>,
    animation: JString<'_>,
) {
    let width = width as usize;
    let height = height as usize;
    write_instance_into!(inst);

    let frame_count = env.get_array_length(&frames).unwrap();
    let name: String = env.get_string_unchecked(&name).unwrap().into();

    let mut images = Vec::new();

    for i in 0..frame_count {
        let mips: JObjectArray<'_> = env.get_object_array_element(&frames, i).unwrap().into();
        let image: JIntArray = env.get_object_array_element(&mips, 0).unwrap().into();

        let frame = {
            let pixels = env
                .get_array_elements(&image, ReleaseMode::NoCopyBack)
                .unwrap();

            pixels.to_owned()
        };

        if width * height != frame.len() {
            tracing::error!(
                what = "a frame had the wrong size image",
                resource = name,
                frame = i,
                width,
                height,
                expect_pixels = width * height,
                actual_pixels = frame.len()
            );
            throw!(
                env,
                inst.textures
                    .borrow_mut()
                    .enqueue_sprite(name, TextureImage::None)
            );
            return;
        }

        let mut image = RgbaImage::new(width as u32, height as u32);

        for (pixel, argb) in frame.iter().enumerate() {
            let [a, r, g, b] = argb.to_be_bytes();

            image.put_pixel(
                (pixel % width) as u32,
                (pixel / width) as u32,
                Rgba([r, g, b, a]),
            );
        }

        images.push(image);
    }

    if images.len() > 1 {
        if let Some(animation) = get_animation_metadata(&mut env, animation) {
            throw!(
                env,
                inst.textures.borrow_mut().enqueue_sprite(
                    name,
                    TextureImage::Frames {
                        width: width as u32,
                        height: width as u32,
                        frames: images,
                        animation,
                    },
                )
            );
        } else {
            tracing::error!(
                what = "a spritesheet texture didn't have an animation: missingno will be used instead",
                resource = name,
            );
            throw!(
                env,
                inst.textures
                    .borrow_mut()
                    .enqueue_sprite(name, TextureImage::None)
            );
        };
    } else {
        throw!(
            env,
            inst.textures.borrow_mut().enqueue_sprite(
                name,
                TextureImage::Static {
                    image: images.remove(0),
                },
            )
        );
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueRawSprite(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    name: JString<'_>,
    image: JByteBuffer<'_>,
    animation: JString<'_>,
) {
    write_instance_into!(inst);

    let image = std::slice::from_raw_parts(
        env.get_direct_buffer_address(&image).unwrap(),
        env.get_direct_buffer_capacity(&image).unwrap(),
    );

    throw!(
        env,
        inst.textures.borrow_mut().enqueue_sprite(
            env.get_string_unchecked(&name).unwrap().into(),
            TextureImage::Data {
                data: image.to_owned(),
                animation: get_animation_metadata(&mut env, animation),
            },
        )
    );
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn beginTextureReload(_: JNIEnv<'_>, _: JClass<'_>) {
    write_instance_into!(inst);

    inst.textures.borrow_mut().begin_texture_reload();
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn finishTextureReload(mut env: JNIEnv<'_>, _: JClass<'_>) {
    write_instance_into!(inst);

    throw!(env, inst.textures.borrow_mut().finish_texture_reload());
}
