# Fluorescent Engine - Phase 3 Multi-Agent Prompts

## 1. Rendering Architect Agent
**Task:** Implement Advanced Lighting (Dynamic GI, Virtual Shadow Maps, and Upscaling)
**Prompt:**
> "As the Rendering Architect Agent for the Fluorescent Engine, your goal is to advance the rendering pipeline to Phase 3 AAA standards. Please implement Virtual Shadow Maps (VSM) and Dynamic Global Illumination (Hardware/Screen-space RTGI) within the `fluorite_core` and `fluoderpod_render` modules. Additionally, integrate a framework for temporal upscaling techniques (TAA, FSR2) into the core render graph. Ensure all lighting shaders (`pbr_forward.wgsl`, `shadow_depth.wgsl`) correctly integrate these new features while maintaining high-performance on Vulkan/WebGPU."

## 2. Systems Programmer Agent
**Task:** Refactor core ECS and Allocators for massive World Streaming workloads
**Prompt:**
> "As the Systems Programmer Agent, your primary directive is to support the Phase 3 World Streaming architectures. Please refactor the existing ECS (`fluorite_core/src/ecs/`) and memory allocators (`fluorite_core/src/allocator/`) to safely and efficiently handle asynchronous chunk streaming for massive open-world scales. Ensure the architecture supports loading/unloading massive data chunks in background threads without causing main-thread stutter, and verify that the Zero-Copy FFI bridge scales perfectly with these massive entity loads."

## 3. Gameplay/Simulation Agent
**Task:** Implement Procedural Content Generation (PCG) and Advanced Animation/Destruction
**Prompt:**
> "As the Gameplay/Simulation Agent, your goal is to build out the Phase 3 gameplay systems. Please implement a Procedural Content Generation (PCG) framework in `fluorite_core/src/streaming/` capable of handling terrain generation, vegetation scattering, and biome mapping. Secondly, in `fluorite_core/src/physics/` and `animation/`, implement runtime bone/skeletal IK constraints and integrate physics-driven destruction logic (Chaos-style chunk separation and dynamic rigid body spawning via Rapier3D)."

## 4. Networking & Cloud Agent
**Task:** Finalize Phase 3 Multiplayer Sync readiness for Phase 4
**Prompt:**
> "As the Networking & Cloud Agent, you need to finalize the multiplayer replication sync within the Rust networking foundation (`fluorite_core/src/net/` and `fluoderpod_render/src/reticulum_bridge.rs`). Implement robust client-side position interpolation and extrapolation for high-latency connections, and build out the entity culling logic on the server to only process and stream network updates for entities within a player's visible radius."

## 5. Tools & Editor Agent
**Task:** Build Editor Tools for Phase 3 Features (VFX and PCG Nodes)
**Prompt:**
> "As the Tools & Editor Agent, your task is to expand the `fluorite_editor` Flutter desktop tooling to support the new Phase 3 features. Implement a visual node-based editor for the GPU particle simulation framework and the new Procedural Content Generation (PCG) systems. Ensure the UI can seamlessly serialize/deserialize node graphs into the engine's JSON/YAML Render Graph data structure."

## 6. UI/UX & Mobile Integration Agent
**Task:** Finalize Phase 2/3 Mobile Functional Test Apps
**Prompt:**
> "As the UI/UX & Mobile Integration Agent, ensure the Phase 3 features render beautifully in the mobile Flutter application (`functional_test_app`). Please finalize the Social Feed and Live Social Map integrations. Ensure the engine properly transitions to extreme low-power modes (Thermal Scaling to 15fps) when the user is simply reading the social feed or navigating the 2D UI."

## 7. Compute Culling Agent
**Task:** Finalize GPU Compute Culling Optimization
**Prompt:**
> "As the Compute Culling Agent, verify and optimize the Phase 3 frustum and occlusion culling compute shaders in `fluoderpod_render/src/culling/`. Ensure the bounding sphere frustum tests and the Hierarchical Z-Buffer (HZB) occlusion passes dynamically ingest batches from `fluoderpod` and successfully populate the indirect draw buffer without any CPU readbacks."

## 8. Virtual Geometry Agent
**Task:** Finalize Nanite-style Virtual Geometry Streaming
**Prompt:**
> "As the Virtual Geometry Agent, finish the Phase 3 Nanite-style micro-polygon rendering system in `fluoderpod_render/src/virtual_geometry/`. Ensure the `cluster_lod.wgsl` compute shader correctly evaluates screen-space error across clusters, and that the async streaming system effectively pages `ClusterHeader` and payload data from disk to GPU memory based on the visibility pass."

## 9. Unified Pipeline Agent
**Task:** GPU VFX Framework and Command Buffer Unification
**Prompt:**
> "As the Unified Pipeline Agent, your mission is two-fold for Phase 3. First, verify the transition to unified GPU command buffers in `fluoderpod_render/src/unified_pipeline/` is completely minimizing CPU submission overhead via `draw_indexed_indirect`. Second, implement the architecture for a node-based GPU particle simulation framework (Niagara-style) that operates entirely within the unified compute pipeline and interacts with the GBuffer/Depth buffers."
