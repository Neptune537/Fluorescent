# Project: Fluorescent Engine — Master Architecture & Milestones

## Architecture Overview
Fluorescent combines a high-performance native Rust core (`fluorite_core`, `fluoderpod_render`) with an ergonomic Flutter editor and runtime layer (`fluorite_editor`, `fluorescent_flame`, `fluorescent_ecs`).
- **Target Platforms:** Android (Vulkan NDK / `AHardwareBuffer`) and Web (Wasm / WebGPU).
- **Rendering:** Clustered Forward+ & Phase 3 GPU-driven pipeline supporting compute frustum & HZB occlusion culling, Nanite-style virtual geometry cluster evaluation, Cook-Torrance PBR, 4-split CSM, and unified indirect draw calls.
- **Spatial:** 32-byte cache-aligned Bounding Volume Hierarchy (BVH) with 16-bin SAH construction, SIMD slab raycasting, and hierarchical frustum culling (<0.2ms for 10k entities).
- **Physics:** Pure Rust `rapier3d` integration with fixed 60Hz timestep, continuous collision detection (CCD), Kinematic Character Controller (KCC), Pacejka vehicle dynamics, and Euphoria-class biomechanical ragdolls.
- **Memory & Zero-Copy:** Double-buffered 16MB frame arena with 64-byte hardware cache alignment, 0xAA/0x55 boundary sentinels, and direct `AHardwareBuffer` / WebGPU canvas texture sharing.
- **Networking & VR:** Reticulum Network Stack (RNS) native mesh bridge, Nexus Protocol (Telemetry, TensorDiff), and OpenXR VR for Meta Quest 3.

---

## Feature Inventory

| # | Subsystem / Feature | Description | Status |
|---|---|---|---|
| 1 | **PBR Metallic-Roughness BRDF** | Cook-Torrance microfacet model (GGX $D$, Smith $V$, Schlick $F$), glTF 2.0 conventions | ✅ Completed |
| 2 | **WGSL Shaders Toolchain** | `pbr_forward.wgsl`, `shadow_depth.wgsl`, `cluster_cull.wgsl`, `cluster_lod.wgsl` | ✅ Completed |
| 3 | **Clustered Forward+ Lighting** | 16x9x24 clusters (3,456 cells) supporting 1024+ dynamic lights via compute shader & CPU fallback | ✅ Completed |
| 4 | **Advanced Lighting (GI & VSM)** | Virtual Shadow Maps, Hardware RTGI, Irradiance Fields | 🚧 Partially Completed |
| 5 | **32-Byte FlatBvhNode & 16-Bin SAH** | glam::Vec3A, 32-byte node layout, 2 nodes per 64-byte cache line, SIMD slab raycasting | ✅ Completed |
| 6 | **Hierarchical Box-Frustum Culling** | Center-extents box-plane tests with inside-inheritance (<0.2ms for 10k entities) | ✅ Completed |
| 7 | **Compute Frustum & HZB Occlusion** | GPU compute shader frustum culling and Hierarchical Z-Buffer depth downsampling & culling | ✅ Completed |
| 8 | **Virtual Geometry (Nanite-Style)** | Screen-space cluster LOD evaluation compute shader (`cluster_lod.wgsl`), asynchronous streaming | ✅ Completed |
| 9 | **Unified Indirect GPU Command Buffers**| Direct GPU indirect draw buffer submissions (`submit_indirect_draws`) | ✅ Completed |
| 10 | **Mobile Thermal & Power Scaler** | Dynamic framerate throttling: Active (60 FPS) $\to$ Static (30 FPS) $\to$ Idle (15 FPS) | ✅ Completed |
| 11 | **Rapier3D Physics & 60Hz Accumulator** | Deterministic physics pipeline with continuous collision detection (CCD) | ✅ Completed |
| 12 | **Kinematic Character Controller (KCC)**| Autostep stairs, slope sliding limits, ground snapping, dynamic obstacle response | ✅ Completed |
| 13 | **Advanced Animation & Destruction** | IK Solvers, FABRIK, Chaos-style Destruction | ❌ Not Started |
| 14 | **Procedural AI & Crowd Simulation** | Batch Line-of-Sight, Crowd Density Flow Fields, Daily NPC Schedules, Police Response Tree | ❌ Not Started |
| 15 | **Ray-Traced Spatial Acoustics** | Sound reflection/transmission paths, Sabine RT60 physical reverberation modeling | ✅ Completed |
| 16 | **World & Virtual Texture Streaming** | Asynchronous NVMe/disk world streamer, sparse page table virtual texturing with GPU feedback | 🚧 Partially Completed |
| 17 | **Reticulum Mesh Bridge & Nexus Protocol**| Native decentralized mesh networking (`NexusMessage::Telemetry`, `TensorDiff`, `Ping`) | ✅ Completed |
| 18 | **OpenXR / Meta Quest 3 Integration** | Direct Vulkan swapchain rendering via `wgpu_hal`, 90/120Hz frame pacing, head & hand tracking | ✅ Completed |
| 19 | **Zero-Copy Memory Arenas & Sentinel** | DoubleBufferedFrameAllocator, ArenaAllocator, 0xAA/0x55 sentinels, GcFirewall | ✅ Completed |
| 20 | **Flutter Flame 2D/3D Viewport** | `FluorescentViewport` component, zero-copy texture bridge, 2D/3D compositing | ✅ Completed |
| 21 | **Creator Tools & Asset Pipeline** | `.gltf` $\to$ `.fworld` compiler, Blender live WebSocket sync, Higgsfield generative AI bridge | ✅ Completed |
| 22 | **Multi-Tier Automated Test Suites** | Tiers 1-4 coverage, boundary limits, cross-feature integration, and 1,000-frame stability tests | ✅ Completed |

---

## Documentation Links
- **[Features & Implementation Guide](docs/FEATURES_AND_IMPLEMENTATION_GUIDE.md)**
- **[Engine Specification](docs/ENGINE_SPECIFICATION.md)**
- **[GPU-Driven Architecture](docs/GPU_DRIVEN_ARCHITECTURE.md)**
- **[OpenXR Integration Guide](docs/OPENXR_INTEGRATION.md)**
- **[Reticulum Mesh Bridge](docs/RETICULUM_BRIDGE.md)**
