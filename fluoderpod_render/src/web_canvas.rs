//! WebAssembly & WebGPU bindings for the Fluoderpod render engine.
//!
//! Provides the browser entrypoint mounted by Jaspr or Flutter Web:
//! binds to the HTML5 `<canvas id="...">` element, requests WebGPU adapter/device,
//! configures a swapchain surface, and schedules the continuous render loop
//! using the browser's `requestAnimationFrame` API.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::thermal;
use crate::unified_pipeline::PipelineManager;
use crate::FluoderpodRenderer;

thread_local! {
    /// Currently active renderer instance on the main web thread.
    static ACTIVE_WEB_RENDERER: RefCell<Option<Rc<RefCell<FluoderpodRenderer>>>> = const { RefCell::new(None) };
    /// Surface configuration per canvas id.
    static SURFACE_CONFIGS: RefCell<HashMap<String, (Rc<wgpu::Surface<'static>>, Arc<wgpu::Device>, wgpu::SurfaceConfiguration)>> = RefCell::new(HashMap::new());
    /// Cancellation flags keyed by canvas element id.
    static STOP_FLAGS: RefCell<HashMap<String, Arc<AtomicBool>>> = RefCell::new(HashMap::new());
}

/// Request next browser animation frame via `window.requestAnimationFrame`.
fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(f.as_ref().unchecked_ref());
    }
}

/// Main WebAssembly entrypoint called by Jaspr or JavaScript to initialize WebGPU rendering.
///
/// Binds to `<canvas id="{canvas_id}">`, creates a WebGPU surface, requests a device/queue,
/// sets up the `FluoderpodRenderer`, and runs the non-blocking `requestAnimationFrame` loop.
#[wasm_bindgen]
pub async fn start_fluoderpod_web(canvas_id: &str) -> Result<bool, JsValue> {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&format!("[fluoderpod_render] Initializing WebGPU on canvas '#{}'", canvas_id).into());

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No global browser window found"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document found in browser window"))?;
    let element = document.get_element_by_id(canvas_id).ok_or_else(|| {
        JsValue::from_str(&format!("Canvas element with id '{}' not found in DOM", canvas_id))
    })?;
    let canvas: HtmlCanvasElement = element.dyn_into().map_err(|_| {
        JsValue::from_str(&format!("Element with id '{}' is not an HTMLCanvasElement", canvas_id))
    })?;

    let width = canvas.width().max(1);
    let height = canvas.height().max(1);

    // Initialize wgpu instance targeting WebGPU / WebGL backends in the browser
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL | wgpu::Backends::BROWSER_WEBGPU,
        flags: wgpu::InstanceFlags::default(),
        dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("Failed to create wgpu surface on canvas: {:?}", e)))?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| JsValue::from_str("Failed to find suitable WebGPU/WebGL adapter"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Fluoderpod WebGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to request WebGPU device: {:?}", e)))?;

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(caps.formats.first().copied().unwrap_or(wgpu::TextureFormat::Rgba8Unorm));

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto),
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let surface_rc = Rc::new(surface);
    SURFACE_CONFIGS.with(|sc| {
        sc.borrow_mut().insert(canvas_id.to_string(), (surface_rc.clone(), device.clone(), config.clone()));
    });

    let pipeline_manager = PipelineManager::new(device.clone(), queue.clone(), None, None, None);
    let renderer = Rc::new(RefCell::new(FluoderpodRenderer::new(pipeline_manager)));

    ACTIVE_WEB_RENDERER.with(|r| {
        *r.borrow_mut() = Some(renderer.clone());
    });

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    STOP_FLAGS.with(|flags| {
        flags.borrow_mut().insert(canvas_id.to_string(), stop_flag);
    });

    // Wire up non-blocking requestAnimationFrame render loop
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let device_clone = device.clone();
    let queue_clone = queue.clone();
    let surface_for_loop = surface_rc.clone();
    let canvas_id_owned = canvas_id.to_string();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if stop_flag_clone.load(Ordering::Relaxed) {
            web_sys::console::log_1(&format!("[fluoderpod_render] Stop flag detected for '#{}'", canvas_id_owned).into());
            return;
        }

        // Honor thermal throttle and idle flags
        if !thermal::is_idle() && thermal::effective_fps() > 0 {
            let surface_texture = match surface_for_loop.get_current_texture() {
                Ok(t) => t,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    SURFACE_CONFIGS.with(|sc| {
                        if let Some((s, d, conf)) = sc.borrow().get(&canvas_id_owned) {
                            s.configure(d, conf);
                        }
                    });
                    request_animation_frame(f.borrow().as_ref().unwrap());
                    return;
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    request_animation_frame(f.borrow().as_ref().unwrap());
                    return;
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("WebGPU surface acquisition error: {:?}", e).into());
                    return;
                }
            };

            let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = device_clone.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Fluoderpod Web Frame Encoder"),
            });

            if let Ok(mut r) = renderer.try_borrow_mut() {
                r.render_frame_to_view(&mut encoder, &view);
            }

            queue_clone.submit(std::iter::once(encoder.finish()));
            surface_texture.present();
        }

        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
    web_sys::console::log_1(&format!("[fluoderpod_render] WebGPU render loop started successfully on '#{}'", canvas_id).into());
    Ok(true)
}

/// Backward-compatible alias for earlier stubs.
#[wasm_bindgen]
pub async fn start_web_renderer(canvas_id: String) -> Result<(), JsValue> {
    start_fluoderpod_web(&canvas_id).await.map(|_| ())
}

/// Stop the render loop associated with a canvas element.
#[wasm_bindgen]
pub fn stop_fluoderpod_web(canvas_id: &str) -> bool {
    STOP_FLAGS.with(|flags| {
        if let Some(flag) = flags.borrow_mut().remove(canvas_id) {
            flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    })
}

/// Resize the WebGPU swapchain surface when the canvas dimensions change.
#[wasm_bindgen]
pub fn resize_fluoderpod_web(canvas_id: &str, width: u32, height: u32) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    SURFACE_CONFIGS.with(|sc| {
        let mut map = sc.borrow_mut();
        if let Some((surface, device, config)) = map.get_mut(canvas_id) {
            config.width = width;
            config.height = height;
            surface.configure(device, config);
            true
        } else {
            false
        }
    })
}

/// Ingests a raw contiguous buffer of 64-byte `EntityPacket64` structs on Web.
/// Copies from Uint8Array/&[u8] into the active `FluoderpodRenderer` buffers.
/// Returns entity count on success, -1 if no active renderer, -2 if invalid length.
#[wasm_bindgen]
pub fn fluoderpod_ingest_batch_web(data: &[u8]) -> i32 {
    ACTIVE_WEB_RENDERER.with(|r| {
        if let Some(renderer_rc) = r.borrow().as_ref() {
            if let Ok(mut renderer) = renderer_rc.try_borrow_mut() {
                match renderer.ingest_entity_packets(data) {
                    Ok(count) => i32::try_from(count).unwrap_or(i32::MAX),
                    Err(_) => -2,
                }
            } else {
                -1
            }
        } else {
            -1
        }
    })
}

/// Update target FPS from Web/Jaspr.
#[wasm_bindgen]
pub fn fluoderpod_set_target_fps_web(fps: u32) {
    thermal::set_target_fps(fps);
}

/// Update idle state from Web/Jaspr.
#[wasm_bindgen]
pub fn fluoderpod_set_idle_web(idle: bool) {
    thermal::set_idle(idle);
}
