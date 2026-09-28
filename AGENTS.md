# Agent Roles

To effectively implement this complex architecture, we define the following agent roles for concurrent multi-agent execution:

1. **Rendering Architect Agent**: Focuses on the core graphics pipeline, including the Vulkan/Metal bindings, the Render Graph, and Shader Toolchain (Phase 2 and Phase 3 features like dynamic quality tiers and GPU VFX).
2. **Systems Programmer Agent**: Responsible for the underlying Rust core foundation, ECS, memory management, and `flutter_rust_bridge` zero-copy communication.
3. **Gameplay/Simulation Agent**: Implements the Physics, Navigation, Animation, and Procedural Content Generation (PCG) systems.
4. **Networking & Cloud Agent**: Handles Phase 4 requirements, including multiplayer replication, rollback, dedicated server logic, and cloud integrations (e.g., Firebase, OpenRouteService for mapping).
5. **Tools & Editor Agent**: Develops the Flutter-based Editor UI, Visual Scripting, Profiler, and Asset Pipeline tooling (Phase 5).
6. **UI/UX & Mobile Integration Agent**: Focuses on Flutter UI application layers, including social feeds, background location tracking, battery-efficient GPS, and push notifications.

## Phase 3 GPU-Driven Rendering Sub-Agents

To parallelize the advanced GPU-driven rendering pipeline developed inside `fluoderpod_render`, we introduce specialized sub-agents:

7. **Compute Culling Agent**: Dedicated to designing and implementing compute shader-based frustum and occlusion culling, integrating with the high-throughput `fluoderpod` FFI batching to keep the CPU out of the loop.
8. **Virtual Geometry Agent**: Focused on building the Nanite-style micro-polygon rendering system, streaming, and LOD selection on the GPU.
9. **Unified Pipeline Agent**: Responsible for transitioning the engine architecture to unified GPU command buffers, minimizing CPU submission overhead, and defining the overarching general API structures across Vulkan, Metal, and WebGPU.

---

## Target Scope & Critical Priority Fixes

> [!IMPORTANT]
> **TARGET PLATFORMS: ANDROID AND WEB ONLY**
> All engineering, rendering, build pipelines, and integrations in this repository are strictly focused on **Android** and **Web**.
> * **Android:** Uses Android NDK, Vulkan rendering (`fluoderpod_render` / `android_vulkan.rs`), and Android SDK APIs.
> * **Web:** Uses Flutter Web, WebAssembly (Wasm), and WebGPU / CanvasKit.
> * iOS, macOS, and Desktop are deprioritized and out of scope.

### Prioritized Fluoderpod Fixes Backlog
1. **Android Vulkan Texture Registry Bridge (`android_vulkan.rs`):**
   * Link Vulkan render targets directly to Flutter Android external texture registry using zero-copy `ANativeWindow` or Android `HardwareBuffer`.
   * Eliminate CPU frame read-backs to ensure 60+ FPS on mobile hardware.
2. **WebAssembly + WebGPU Pipeline (`--target wasm32-unknown-unknown`):**
   * Ensure compilation to Wasm with WebGPU backend without panicking on missing browser APIs.
   * Provide clean JS/Wasm canvas binding for Flutter Web.
3. **Compute Frustum & Occlusion Culling (`culling/mod.rs`):**
   * Finalize compute shader frustum culling and Hierarchical Z-Buffer (HZB) occlusion culling.
   * Keep CPU out of draw-call culling and geometry sorting when processing large batches from the client app.
4. **Virtual Geometry / Nanite-Style LOD (`virtual_geometry/mod.rs`):**
   * Implement GPU cluster evaluation and continuous LOD streaming for dense map terrain and constellation views.
5. **Mobile Thermal & Power Scaling:**
   * Dynamic frame rate throttling (e.g., dropping from 60 FPS down to 15-30 FPS or idle when static) to eliminate overheating and battery drain.
