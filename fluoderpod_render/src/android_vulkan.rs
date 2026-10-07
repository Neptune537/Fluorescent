#![cfg(target_os = "android")]

use jni::objects::{JClass, JObject};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use log::{error, info, warn};
use ndk_sys::{
    ANativeWindow, ANativeWindow_acquire, ANativeWindow_release,
    AHardwareBuffer, AHardwareBuffer_Desc, AHardwareBuffer_allocate, AHardwareBuffer_release,
};
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::thermal;
use crate::unified_pipeline::PipelineManager;
use crate::FluoderpodRenderer;

const AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE: u64 = 1 << 8;
const AHARDWAREBUFFER_USAGE_GPU_COLOR_OUTPUT: u64 = 1 << 9;
const AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM: u32 = 1;

/// Represents an active Vulkan swapchain render session running on an ANativeWindow.
struct AndroidRenderSession {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    window: usize,
    renderer: Arc<Mutex<FluoderpodRenderer>>,
}

static SESSIONS: Mutex<Option<HashMap<jlong, AndroidRenderSession>>> = Mutex::new(None);

/// Ingests entities into any active Android Vulkan session.
pub fn ingest_into_any_active_session(raw: &[u8]) -> i32 {
    let mut lock = SESSIONS.lock().unwrap();
    if let Some(map) = lock.as_mut() {
        for session in map.values() {
            if let Ok(mut r) = session.renderer.lock() {
                match r.ingest_entity_packets(raw) {
                    Ok(count) => return i32::try_from(count).unwrap_or(i32::MAX),
                    Err(_) => return -2,
                }
            }
        }
    }
    -1
}

/// Ingests entities into a specific window's active session.
pub fn ingest_into_session(window: jlong, raw: &[u8]) -> i32 {
    let mut lock = SESSIONS.lock().unwrap();
    if let Some(map) = lock.as_mut() {
        if let Some(session) = map.get(&window) {
            if let Ok(mut r) = session.renderer.lock() {
                match r.ingest_entity_packets(raw) {
                    Ok(count) => return i32::try_from(count).unwrap_or(i32::MAX),
                    Err(_) => return -2,
                }
            }
        }
    }
    -1
}

#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_createHardwareBuffer<'local>(
    mut _env: jni::EnvUnowned<'local>,
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
    mut _env: jni::EnvUnowned<'local>,
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

pub unsafe fn import_hardware_buffer_to_wgpu(
    device: &wgpu::Device,
    _buffer: *mut AHardwareBuffer,
    _width: u32,
    _height: u32,
) -> Option<wgpu::Texture> {
    device.as_hal::<wgpu_hal::api::Vulkan, _, _>(|hal_device| {
        if let Some(_vk_device) = hal_device {
            // Extension point for direct AHardwareBuffer Vulkan import
        }
    });
    None
}

/// Starts the Fluoderpod Vulkan rendering pipeline on the Android ANativeWindow.
///
/// Matches Kotlin: `external fun startRenderer(window: Long, width: Int, height: Int): Boolean`
#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_startRenderer<'local>(
    mut _env: jni::EnvUnowned<'local>,
    _class: JObject<'local>,
    window: jlong,
    w: jint,
    h: jint,
) -> jboolean {
    info!(
        "Rust Vulkan: startRenderer called for window 0x{:x} ({}x{})",
        window, w, h
    );

    if window == 0 || w <= 0 || h <= 0 {
        error!("startRenderer called with invalid parameters: window={}, w={}, h={}", window, w, h);
        return JNI_FALSE;
    }

    let native_window = window as *mut ANativeWindow;

    // Acquire an NDK reference to keep the window valid during rendering
    unsafe {
        ANativeWindow_acquire(native_window);
    }

    // 1. Create wgpu Vulkan instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        flags: wgpu::InstanceFlags::default(),
        dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });

    // 2. Create wgpu Surface targeting the Android Native Window
    let handle = AndroidNdkWindowHandle::new(match NonNull::new(native_window as *mut _) {
        Some(nn) => nn,
        None => {
            error!("Null ANativeWindow pointer encountered");
            unsafe { ANativeWindow_release(native_window); }
            return JNI_FALSE;
        }
    });
    let raw_window = RawWindowHandle::AndroidNdk(handle);
    let raw_display = RawDisplayHandle::Android(AndroidDisplayHandle::new());

    let surface_target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: raw_display,
        raw_window_handle: raw_window,
    };

    let surface = match unsafe { instance.create_surface_unsafe(surface_target) } {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create wgpu surface from ANativeWindow: {:?}", e);
            unsafe { ANativeWindow_release(native_window); }
            return JNI_FALSE;
        }
    };

    // 3. Request Vulkan adapter
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    })) {
        Some(a) => a,
        None => {
            error!("Failed to acquire Vulkan adapter for surface");
            unsafe { ANativeWindow_release(native_window); }
            return JNI_FALSE;
        }
    };

    // 4. Request Device & Queue
    let (device, queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Fluoderpod Android Vulkan Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        },
        None,
    )) {
        Ok((d, q)) => (Arc::new(d), Arc::new(q)),
        Err(e) => {
            error!("Failed to acquire Vulkan device/queue: {:?}", e);
            unsafe { ANativeWindow_release(native_window); }
            return JNI_FALSE;
        }
    };

    // 5. Configure Swapchain Surface
    let width = w as u32;
    let height = h as u32;
    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps.formats.iter().copied()
        .find(|f| f.is_srgb())
        .unwrap_or(caps.formats.first().copied().unwrap_or(wgpu::TextureFormat::Rgba8Unorm));

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Opaque),
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // 6. Initialize FluoderpodRenderer
    let pipeline_manager = PipelineManager::new(device.clone(), queue.clone(), None, None, None);
    let renderer = Arc::new(Mutex::new(FluoderpodRenderer::new(pipeline_manager)));

    let running = Arc::new(AtomicBool::new(true));
    let running_loop = running.clone();
    let renderer_loop = renderer.clone();
    let device_loop = device.clone();
    let queue_loop = queue.clone();

    // 7. Spawn background render loop
    let thread_handle = match thread::Builder::new()
        .name(format!("vulkan-render-0x{:x}", window))
        .spawn(move || {
            info!("Android Vulkan render loop started for window 0x{:x}", window);
            let mut last_frame = Instant::now();

            while running_loop.load(Ordering::Relaxed) {
                // Honor thermal / idle throttling
                if thermal::is_idle() {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                let target_fps = thermal::effective_fps();
                if target_fps == 0 {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                let frame_budget = Duration::from_micros((1_000_000 / target_fps.max(1)) as u64);
                let elapsed = last_frame.elapsed();
                if elapsed < frame_budget {
                    thread::sleep(frame_budget - elapsed);
                }
                last_frame = Instant::now();

                let surface_texture = match surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Timeout) => continue,
                    Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                        surface.configure(&device_loop, &config);
                        continue;
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        error!("Android Vulkan render loop: OutOfMemory");
                        break;
                    }
                };

                let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = device_loop.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Android Vulkan Encoder"),
                });

                if let Ok(mut r) = renderer_loop.lock() {
                    r.render_frame_to_view(&mut encoder, &view);
                }

                queue_loop.submit(std::iter::once(encoder.finish()));
                surface_texture.present();
            }

            info!("Android Vulkan render loop finished for window 0x{:x}", window);
        })
    {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to spawn render loop thread: {:?}", e);
            unsafe { ANativeWindow_release(native_window); }
            return JNI_FALSE;
        }
    };

    let session = AndroidRenderSession {
        running,
        thread: Some(thread_handle),
        window: window as usize,
        renderer,
    };

    let mut lock = SESSIONS.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);
    if let Some(old) = map.insert(window, session) {
        old.running.store(false, Ordering::SeqCst);
        if let Some(h) = old.thread {
            let _ = h.join();
        }
        unsafe { ANativeWindow_release(old.window as *mut ANativeWindow); }
    }

    info!("Android Vulkan renderer started successfully for window 0x{:x}", window);
    JNI_TRUE
}

/// Stops the Vulkan render loop, cleans up wgpu resources, and releases the ANativeWindow.
///
/// Matches Kotlin: `external fun stopRenderer(window: Long)`
#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_stopRenderer<'local>(
    mut _env: jni::EnvUnowned<'local>,
    _class: JObject<'local>,
    window: jlong,
) {
    info!("Rust Vulkan: stopRenderer called for window 0x{:x}", window);
    let session = {
        let mut lock = SESSIONS.lock().unwrap();
        lock.as_mut().and_then(|map| map.remove(&window))
    };

    if let Some(session) = session {
        session.running.store(false, Ordering::SeqCst);
        if let Some(h) = session.thread {
            let _ = h.join();
        }
        unsafe {
            ANativeWindow_release(session.window as *mut ANativeWindow);
        }
        info!("Rust Vulkan: stopped and released window 0x{:x}", window);
    } else {
        warn!("Rust Vulkan: stopRenderer called on inactive window 0x{:x}", window);
    }
}

/// JNI bridge to ingest a batch of 64-byte entity packets into a window's renderer.
#[no_mangle]
pub extern "system" fn Java_com_fluorescent_vulkan_FluorescentVulkanPlugin_ingestBatch<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: JObject<'local>,
    window: jlong,
    buffer: jni::objects::JByteArray<'local>,
) -> jint {
    let bytes_res = env.with_env(|env| env.convert_byte_array(&buffer));
    let bytes = match bytes_res.into_outcome() {
        jni::Outcome::Ok(b) => b,
        jni::Outcome::Err(e) => {
            error!("Failed to convert byte array from JNI: {:?}", e);
            return -1;
        }
        jni::Outcome::Panic(_) => {
            error!("Panic while converting byte array from JNI");
            return -1;
        }
    };
    ingest_into_session(window, &bytes)
}
