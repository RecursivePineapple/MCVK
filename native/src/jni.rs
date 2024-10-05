use std::cell::RefCell;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;

use anyhow::Context;

use crate::vulkan::glfw_window::CreateWindowSurface;
use crate::vulkan::glfw_window::GLFWFns;
use crate::vulkan::glfw_window::GLFWWindow;
use crate::vulkan::glfw_window::GetPhysicalDevicePresentationSupport;
use crate::vulkan::glfw_window::GetRequiredInstanceExtensions;
use crate::vulkan::glfw_window::GetWindowSize;
use crate::vulkan::sandbox_jni::jni_prelude::*;
use crate::vulkan::swapchain::VsyncMode;

macro_rules! field_cache {
    {name: $name:ident, class: $class:literal, fields: { $($rust_name:ident / $ret_name:ident => $java_name:literal / $sig:literal),* $(,)? }} => {

        struct $name {
            $(pub $rust_name: jni::objects::JFieldID,)*
            $(pub $ret_name: jni::signature::ReturnType,)*
        }

        impl $name {
            pub fn get(env: &mut JNIEnv<'_>) -> &'static Self {
                static CACHE: std::sync::OnceLock<$name> = std::sync::OnceLock::new();

                CACHE.get_or_init(|| {
                    let class = env.find_class($class).unwrap();
                    Self {
                        $($rust_name: env.get_field_id(&class, $java_name, $sig).unwrap(),)*
                        $($ret_name: <jni::signature::ReturnType as std::str::FromStr>::from_str($sig).unwrap(),)*
                    }
                })
            }
        }
    };
}

macro_rules! throw {
    ($env:expr, $res:expr) => {
        match { $res } {
            Ok(x) => x,
            Err(e) => {
                $env.throw_new("java/lang/RuntimeException", e.to_string())
                    .unwrap();

                return Default::default();
            }
        }
    };
}

macro_rules! jni_bail {
    ($env:expr, $message:expr) => {
        $env.throw_new("java/lang/RuntimeException", ($message).to_string())
            .unwrap();

        return Default::default();
    };
}

macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

macro_rules! jni_todo {
    ($env:expr, $message:literal) => {
        $env.throw_new("java/lang/RuntimeException", ($message).to_string())
            .unwrap();

        return Default::default();
    };
    ($env:expr) => {
        $env.throw_new(
            "java/lang/RuntimeException",
            format!("{} is not yet implemented", function!()),
        )
        .unwrap();

        return Default::default();
    };
}

pub static INSTANCE: RwLock<Option<MCVK>> = RwLock::new(None);

macro_rules! read_instance_into {
    ($var:ident) => {
        let $var = crate::jni::INSTANCE.read().unwrap();
        let $var = ($var).as_ref().unwrap();
    };
}

macro_rules! write_instance_into {
    ($var:ident) => {
        let mut $var = crate::jni::INSTANCE.write().unwrap();
        let $var: &mut MCVK = ($var).as_mut().unwrap();
    };
}

macro_rules! read_field_into {
    ($var:ident ; $($field:ident);*) => {
        let $var = crate::jni::INSTANCE.read().unwrap();
        let $var = ($var).as_ref().unwrap();
        $(let $field = $var.$field.borrow();)*
    };
}

macro_rules! write_field_into {
    ($var:ident ; $($field:ident);*) => {
        let $var = crate::jni::INSTANCE.read().unwrap();
        let $var = ($var).as_ref().unwrap();
        $(let mut $field = $var.$field.borrow_mut();)*
    };
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
unsafe fn initialize(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    window_ptr: jlong,
    get_required_instance_extensions: GetRequiredInstanceExtensions,
    get_physical_device_presentation_support: GetPhysicalDevicePresentationSupport,
    create_window_surface: CreateWindowSurface,
    get_window_size: GetWindowSize,
) {
    let mut l = INSTANCE.write().unwrap();

    if l.is_some() {
        jni_bail!(env, "instance was already initialized");
    }

    let window = Arc::new(RefCell::new(GLFWWindow {
        glfw: GLFWFns {
            get_required_instance_extensions,
            get_physical_device_presentation_support,
            create_window_surface,
            get_window_size,
        },
        window: window_ptr as *mut glfw::ffi::GLFWwindow,
    }));

    let inst = MCVK::new(window);

    let inst = throw!(env, inst.context("could not create vulkan instance"));

    l.replace(inst);
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn cleanup(_: JNIEnv<'_>, _: JClass<'_>) {
    INSTANCE.write().unwrap().take();
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn setMaxFPS(_: JNIEnv<'_>, _: JClass<'_>, max_fps: jint) {
    write_instance_into!(inst);

    inst.set_max_fps(if max_fps <= 0 {
        None
    } else {
        Some(max_fps as u32)
    });
}

#[jni_export("com.recursive_pineapple.mcvk.rendering.MCVKNative")]
pub unsafe fn setVsyncMode(_: JNIEnv<'_>, _: JClass<'_>, vsync_mode: jint) {
    write_instance_into!(inst);

    inst.set_vsync(VsyncMode::from_i32(vsync_mode).unwrap());
}
