#![cfg(target_os = "android")]

use jni::objects::{JClass, JObject};
use jni::sys::{jint, jlong};
use log::{error, info};
use ndk_sys::{
    AHardwareBuffer, AHardwareBuffer_Desc, AHardwareBuffer_allocate, AHardwareBuffer_release,
};
use std::ptr;

// Some constants for usage may not be directly available in older NDK-sys versions as enums,
// using raw values corresponding to AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE and GPU_COLOR_OUTPUT
const AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE: u64 = 1 << 8;
const AHARDWAREBUFFER_USAGE_GPU_COLOR_OUTPUT: u64 = 1 << 9;
const AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM: u32 = 1;

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_createHardwareBuffer<'local>(
    mut _env: jni::JNIEnv<'local>,
    _class: JObject<'local>,
    width: jint,
    height: jint,
) -> jlong {
    info!("Rust Vulkan: creating AHardwareBuffer {}x{}", width, height);

    let desc = AHardwareBuffer_Desc {
        width: width as u32,
        height: height as u32,
        layers: 1,
        format: AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM,
        usage: AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE | AHARDWAREBUFFER_USAGE_GPU_COLOR_OUTPUT,
        stride: 0,
        rfu0: 0,
        rfu1: 0,
    };

    let mut buffer: *mut AHardwareBuffer = ptr::null_mut();
    let result = unsafe { AHardwareBuffer_allocate(&desc, &mut buffer) };

    if result != 0 {
        error!("Failed to allocate AHardwareBuffer. Error code: {}", result);
        return 0;
    }

    info!("AHardwareBuffer created successfully in Rust.");

    buffer as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_destroyHardwareBuffer<'local>(
    mut _env: jni::JNIEnv<'local>,
    _class: JObject<'local>,
    buffer_ptr: jlong,
) {
    if buffer_ptr != 0 {
        let buffer = buffer_ptr as *mut AHardwareBuffer;
        unsafe {
            AHardwareBuffer_release(buffer);
        }
        info!("Rust Vulkan: AHardwareBuffer released.");
    }
}

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_createSurfaceFromWindow<'local>(
    env: *mut jni::sys::JNIEnv,
    _class: JClass<'local>,
    surface: JObject<'local>,
) -> jlong {
    use ndk::native_window::NativeWindow;
    let window = unsafe {
        NativeWindow::from_surface(env as *mut _, surface.as_raw())
    };
    match window {
        Some(w) => w.ptr().as_ptr() as jlong,
        None => 0,
    }
}

/// A conceptual outline of how to import the AHardwareBuffer into a wgpu::Texture 
/// via wgpu-hal and Vulkan on Android.
pub unsafe fn import_hardware_buffer_to_wgpu(
    device: &wgpu::Device,
    _buffer: *mut AHardwareBuffer,
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    // This requires wgpu to be compiled with Vulkan backend enabled.
    // To bind an AHardwareBuffer, you must construct a VkImage using 
    // VK_EXTERNAL_MEMORY_HANDLE_TYPE_ANDROID_HARDWARE_BUFFER_BIT_ANDROID
    // and bind the hardware buffer's memory to it.

    device.as_hal::<wgpu_hal::api::Vulkan, _, _>(|hal_device| {
        if let Some(_vk_device) = hal_device {
            // let raw_device = vk_device.raw_device();
            // let android_hardware_buffer_info = vk::ImportAndroidHardwareBufferInfoANDROID {
            //     buffer: _buffer as _,
            //     ..Default::default()
            // };
            // ... Construct vk::Image, allocate memory with android_hardware_buffer_info, and bind.
            
            // For now, this serves as the integration point placeholder.
        }
    });

    // In a full implementation, you would construct a wgpu::Texture from the raw hal::Texture.
    // Since we don't have the full wgpu-core machinery here, we'll return None for now.
    None
}

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_startRenderer<'local>(
    _env: jni::JNIEnv<'local>,
    _class: JObject<'local>,
    _surface: JObject<'local>,
) {
    log::info!("Rust Vulkan: startRenderer called");
    // TODO: Create a Vulkan surface from the Android window and run the render loop
}

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_stopRenderer<'local>(
    _env: jni::JNIEnv<'local>,
    _class: JObject<'local>,
) {
    log::info!("Rust Vulkan: stopRenderer called");
    // TODO: Stop the render loop
}
