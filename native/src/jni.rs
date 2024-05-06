use std::sync::OnceLock;

use crate::vulkan::instance::CreateWindowSurface;
use crate::vulkan::instance::GLFWFns;
use crate::vulkan::instance::GetPhysicalDevicePresentationSupport;
use crate::vulkan::instance::GetRequiredInstanceExtensions;
use crate::vulkan::instance::GetWindowSize;
use crate::vulkan::instance::Sprite;
use crate::vulkan::instance::VsyncMode;
use crate::vulkan::instance::VulkanInstance;
use crate::vulkan::textures::textures::AnimationMetadata;
use enum_primitive::FromPrimitive;
use image::Rgba;
use image::RgbaImage;
use jni::objects::*;
use jni::sys::*;
use jni::JNIEnv;
use native_macros::jni_export;

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
unsafe fn createInstance(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    window_ptr: jlong,
    get_required_instance_extensions: GetRequiredInstanceExtensions,
    get_physical_device_presentation_support: GetPhysicalDevicePresentationSupport,
    create_window_surface: CreateWindowSurface,
    get_window_size: GetWindowSize,
) -> jlong {
    let inst = VulkanInstance::new(
        window_ptr as *mut glfw::ffi::GLFWwindow,
        GLFWFns {
            get_required_instance_extensions,
            get_physical_device_presentation_support,
            create_window_surface,
            get_window_size,
        },
    );

    match inst {
        Ok(inst) => Box::into_raw(Box::new(inst)) as jlong,
        Err(e) => {
            env.throw_new(
                "java/lang/RuntimeException",
                format!("could not create vulkan instance: {e}"),
            )
            .unwrap();

            0
        }
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn destroyInstance(_: JNIEnv<'_>, _: JClass<'_>, ptr: jlong) {
    drop(Box::<VulkanInstance>::from_raw(ptr as *mut VulkanInstance));
}

unsafe fn get_inst(ptr: jlong) -> &'static mut VulkanInstance {
    Box::leak(Box::<VulkanInstance>::from_raw(ptr as *mut VulkanInstance))
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn setMaxFPS(_: JNIEnv<'_>, _: JClass<'_>, ptr: jlong, max_fps: jint) {
    let inst = get_inst(ptr);

    inst.set_max_fps(if max_fps <= 0 {
        None
    } else {
        Some(max_fps as u32)
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn setVsyncMode(_: JNIEnv<'_>, _: JClass<'_>, ptr: jlong, vsync_mode: jint) {
    let inst = get_inst(ptr);

    inst.set_vsync(VsyncMode::from_i32(vsync_mode).unwrap());
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn startFrame(_: JNIEnv<'_>, _: JClass<'_>, ptr: jlong, mc: JObject<'_>) {
    let inst = get_inst(ptr);

    inst.start_frame(mc);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn finishFrame(_: JNIEnv<'_>, _: JClass<'_>, ptr: jlong) {
    let inst = get_inst(ptr);

    inst.finish_frame();
}

macro_rules! field_cache {
    {name: $name:ident, class: $class:literal, fields: { $($rust_name:ident => $java_name:literal / $sig:literal),* $(,)? }} => {

        struct $name {
            $(pub $rust_name: JFieldID),*
        }

        impl $name {
            pub fn get(env: &mut JNIEnv<'_>) -> &'static Self {
                static CACHE: OnceLock<$name> = OnceLock::new();

                CACHE.get_or_init(|| {
                    let class = env.find_class($class).unwrap();
                    Self {
                        $($rust_name: env.get_field_id(&class, $java_name, $sig).unwrap()),*
                    }
                })
            }
        }

    };
}

field_cache! {
    name: AnimationMetadataSectionFields,
    class: "net/minecraft/client/resources/data/AnimationMetadataSection",
    fields: {
        animation_frames => "animationFrames" / "Ljava/util/List;",
        frame_width => "frameWidth" / "I",
        frame_height => "frameHeight" / "I",
        frame_time => "frameTime" / "I",
    }
}

field_cache! {
    name: AnimationFrameFields,
    class: "net/minecraft/client/resources/data/AnimationFrame",
    fields: {
        frame_index => "frameIndex" / "I",
        frame_time => "frameTime" / "I",
    }
}

fn get_animation_metadata(
    env: &mut JNIEnv<'_>,
    animation: JObject<'_>,
) -> Option<AnimationMetadata> {
    None
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueMissingSprite(env: JNIEnv<'_>, _: JClass<'_>, ptr: jlong, name: JString<'_>) {
    let inst = get_inst(ptr);

    inst.enqueue_sprite(
        env.get_string_unchecked(&name).unwrap().into(),
        Sprite::Missing,
    );
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueFrameSprite(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    ptr: jlong,
    name: JString<'_>,
    width: jint,
    height: jint,
    frames: JObjectArray<'_>,
    animation: JObject<'_>,
) {
    let width = width as usize;
    let height = height as usize;
    let inst = get_inst(ptr);

    let frame_count = env.get_array_length(&frames).unwrap();

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
            env.throw_new(
                "java/lang/RuntimeException",
                format!(
                    "frame {i} had the wrong size image at mipmap level 0; width = {width}, height = {height}, expected pixels = {}, actual pixels = {}",
                    width * height,
                    frame.len()
                ),
            )
            .unwrap();
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
        inst.enqueue_sprite(
            env.get_string_unchecked(&name).unwrap().into(),
            Sprite::Frames {
                frames: images,
                animation: get_animation_metadata(&mut env, animation),
            },
        );
    } else {
        inst.enqueue_sprite(
            env.get_string_unchecked(&name).unwrap().into(),
            Sprite::Image {
                image: images.remove(0),
                animation: get_animation_metadata(&mut env, animation),
            },
        );
    }
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn enqueueRawSprite(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    ptr: jlong,
    name: JString<'_>,
    image: JByteBuffer<'_>,
    animation: JObject<'_>,
) {
    let inst = get_inst(ptr);

    let image = std::slice::from_raw_parts(
        env.get_direct_buffer_address(&image).unwrap(),
        env.get_direct_buffer_capacity(&image).unwrap(),
    );

    inst.enqueue_sprite(
        env.get_string_unchecked(&name).unwrap().into(),
        Sprite::Data {
            data: image.to_owned(),
            animation: get_animation_metadata(&mut env, animation),
        },
    );
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn loadSprites(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    ptr: jlong,
    mipmap_levels: jint,
    gen_aniso_data: jboolean,
) {
    let inst = get_inst(ptr);

    if let Err(e) = inst.load_sprites(mipmap_levels as u32, gen_aniso_data == 1) {
        env.throw_new(
            "java/lang/RuntimeException",
            format!("could not load sprites: {e}"),
        )
        .unwrap();
    }
}
