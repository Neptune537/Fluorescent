# OpenXR & VR Integration (`fluoderpod_render`)

## Overview
Fluorescent natively supports OpenXR for Virtual Reality and Mixed Reality applications, specifically targeting headsets like the Meta Quest 3 (Android/Vulkan). Because the `fluoderpod_render` core is written in Rust and utilizes `wgpu-hal`, we bypass Flutter's 2D compositor limitations and render our advanced GPU-driven pipelines directly into the headset's swapchain.

## Architectural Paradigm Shift
When operating in OpenXR mode, the typical Flutter/Flame render loop is inverted to achieve strict latency requirements (90hz/120hz) and maintain head-tracking synchronization.

### 1. OpenXR is the Master
Instead of the `wgpu` instance arbitrarily creating a Vulkan device, OpenXR negotiates the graphics requirements. 
* OpenXR tells us which `VkPhysicalDevice` to use.
* OpenXR mandates specific Vulkan extensions.
* We fetch these requirements using `xr::Instance::vulkan_graphics_requirements2`.

### 2. The `wgpu_hal` Wrapper
Once we have the raw `ash` Vulkan Instance and Device handles dictated by OpenXR, we use `wgpu_hal::api::Vulkan` to wrap these pointers. This allows our `unified_pipeline` to operate exactly as it does in standard 2D/3D mode, completely oblivious to the fact that it is writing into a VR headset's swapchain rather than a standard Flutter texture.

### 3. The Render Loop
The traditional Flame game loop is augmented by the OpenXR frame pacing loop:
* `xrWaitFrame` blocks the thread until the headset is ready for the next frame.
* `xrBeginFrame` signals the start of the rendering logic.
* The `PipelineManager` generates its indirect draw commands and executes the render passes into the OpenXR swapchain images.
* `xrEndFrame` submits the frame for display.

### 4. Flutter/Flame as the Passenger
Flutter continues to handle application lifecycle and user interfaces. 
* Standard Flutter UIs (menus, HUDs) can be rendered to 2D WGPU textures.
* These textures are then composited into the VR scene as floating quads or curved panels by the `fluoderpod_render` engine.
* Tracking data (head poses, controller quaternions/vectors) are streamed from Rust back *up* to Dart via `flutter_rust_bridge` so Flame's gameplay logic can react to physical movements.

## Implementation Details
* **JNI Hooks:** On Android, `MainActivity.kt` explicitly passes down the `JavaVM` and `Activity` pointers via JNI to initialize the OpenXR Android loader.
* **Rust Bindings:** Found in `fluoderpod_render/src/xr.rs` and `fluoderpod_render/src/unified_pipeline/mod.rs` (`init_from_openxr_vulkan`).
* **FFI Layer:** `start_vr_session()` and `stop_vr_session()` are exposed to Flutter.
