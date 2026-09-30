# Fluorescent Engine Specification

**Version:** 0.3.0  
**Status:** Architectural Specification & Implementation Manual  
**Target Platforms:** Android (NDK / Vulkan) & Web (Wasm / WebGPU)  
**Last Updated:** 2026-09-28

---

## 1. What This Engine Is

Fluorescent is a **layered game engine and high-performance runtime platform** combining a native Rust simulation core with Flutter's UI framework, backed by a scalable GPU-driven Vulkan and WebGPU rendering pipeline.

> **"Flutter for the interface. Rust for the heart. Vulkan & WebGPU for the hardware."**

### Core Philosophy

- **Flutter** gives you the interface (Editor, HUD, Menus, Chat, Navigation, Tooling).
- **Rust** gives you the heart (ECS, deterministic physics, memory allocators, jobs, audio, AI).
- **Vulkan / WebGPU** gives you hardware abstraction (zero-copy surfaces, compute culling, virtual geometry).
- **Godot** gives lessons in developer experience.
- **O3DE** gives lessons in modular architecture.
- **Unreal** gives the benchmark for AAA rendering capabilities.

---

## 2. High-Level Architecture

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        FLUTTER & FLAME LAYER (DART)                    │
│  - Editor UI, In-Game HUDs, Menus, Chat, Navigation, Social Layers     │
│  - Flame Component Tree & 2D Sprites/Overlays                          │
│  - Zero-Copy `Texture` Widget Bridge (`FluorescentViewport`)           │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Zero-Copy FFI / Shared Memory
┌───────────────────────────────────▼────────────────────────────────────┐
│                    RUST CORE RUNTIME (`fluorite_core`)                 │
│  - Double-Buffered Frame & Arena Allocators (Zero GC, 64B Cache Align) │
│  - Sparse-Set Archetype ECS & High-Throughput Entity Batches           │
│  - Rapier3D Physics, Biomechanics, Aerohydrodynamics                   │
│  - Ray-Traced Acoustics, Sabine RT60 Reverb                            │
│  - Crowd Flow Fields, Line-of-Sight, Dynamic Behavior Trees           │
│  - World Streaming & Virtual Texturing                                 │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Direct Rust-to-GPU Pipeline
┌───────────────────────────────────▼────────────────────────────────────┐
│             GPU-DRIVEN RENDER PIPELINE (`fluoderpod_render`)           │
│  - Compute Frustum Culling & Hierarchical Z-Buffer (HZB) Occlusion     │
│  - Nanite-Style Virtual Geometry Cluster Evaluation & Streaming        │
│  - Unified GPU Command Buffers & Indirect Draw Submissions             │
│  - Adaptive Thermal & Power Scaling (60fps → 30fps → 15fps)            │
│  - Reticulum Mesh Bridge & Nexus Protocol Network Stack                │
│  - Native Android AHardwareBuffer Vulkan Bridge & WebGPU Surface       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
           ┌────────────────────────┴────────────────────────┐
           ▼                                                 ▼
   Android (Vulkan NDK)                             Web (Wasm / WebGPU)
```

---

## 3. Platform Scope & Target Platforms

> [!IMPORTANT]
> **Strict Platform Focus: Android and Web Only**
> All engineering, graphics pipelines, build tooling, and integrations strictly target **Android** and **Web**. Desktop platforms are supported only as developer tools (`fluorite_editor`).
> * **Android:** Built with Android NDK, Vulkan 1.3, and zero-copy `AHardwareBuffer` / `ANativeWindow` texture registry interop (`android_vulkan.rs`). Supports OpenXR for VR headsets like Meta Quest 3.
> * **Web:** Built with `wasm32-unknown-unknown`, WebAssembly SIMD, and WebGPU / CanvasKit canvas bindings (`create_webgpu_surface_from_canvas`).

---

## 4. Two Engines in One: Creation Runtime & Game Runtime

```text
                  FLUORESCENT ENGINE
                         │
            ┌────────────┴────────────┐
            │                         │
       CREATION RUNTIME          GAME RUNTIME
            │                         │
         Flutter                     Rust
            │                         │
         Editor                  ECS / Systems
         Inspector                  Renderer
         Scene tools                Physics
         Material tools             Animation
         VFX tools                  Audio
         World tools                AI
         Profiler                   Networking
            │                         │
            └────────────┬────────────┘
                         │
                   SAME PROJECT
```

---

## 5. Master Subsystems & Feature Breakdown

### 5.1 Core Memory Allocators (`fluorite_core::allocator`)
- **`DoubleBufferedFrameAllocator`**: Swaps between two contiguous 16MB memory blocks each frame. $O(1)$ reset with zero garbage collection.
- **`ArenaAllocator`**: Linear allocation arena with 64-byte hardware cache line alignment.
- **Sentinel Memory Verification**: Guards memory boundaries with `0xAAAAAAAA` header sentinels and `0x55555555` footer sentinels.
- **`GcFirewall`**: Sanitizes Dart/Rust boundary data into Plain Old Data (POD) structs.
- **`SpscRingBuffer`**: Lock-free single-producer single-consumer ring buffer for telemetry streaming.

### 5.2 Sparse-Set ECS (`fluorite_core::ecs` & `packages/fluorescent_ecs`)
- Contiguous `Float32List` / typed data storage.
- Standard 16-float transform stride per entity.
- Movement, Combat, Trap, and Telemetry systems.

### 5.3 GPU-Driven Rendering Pipeline (`fluoderpod_render`)
- **48-Byte `PackedEntityInstance`**: High-throughput ingestion format (`position`, `sphere_radius`, `rotation_quat`, `scale`, `cluster_and_flags`).
- **Compute Frustum Culling (`frustum_culling.wgsl`)**: Tests entity bounding spheres against 6 camera frustum planes on the GPU.
- **HZB Occlusion Culling (`hzb_culling.wgsl`, `hzb_downsample.wgsl`)**: Evaluates depth occlusion against a Hierarchical Z-Buffer mip pyramid.
- **Virtual Geometry (Nanite-Style) (`cluster_lod.wgsl`, `mesh_loader.rs`)**: Evaluates screen-space geometric error per triangle cluster (64-128 triangles) and streams LODs asynchronously.
- **Unified Indirect Draw (`submit_indirect_draws`)**: Single indirect draw command execution reading arguments directly populated by compute shaders.
- **Adaptive Thermal Scaler (`thermal/mod.rs`)**: Throttles frame presentation based on scene activity (60 FPS Active $\to$ 30 FPS Static $\to$ 15 FPS Idle).

### 5.4 Spatial Indexing & BVH (`fluorite_core::spatial`)
- 32-byte `FlatBvhNode` (2 nodes per 64-byte cache line).
- 16-bin Surface Area Heuristic (SAH) builder.
- SIMD slab raycasting and hierarchical frustum culling ($<0.2\text{ms}$ for 10,000 entities).

### 5.5 PBR & Clustered Forward+ Renderer (`fluorite_core::rendering`)
- Clustered light assignment (16x9x24 = 3,456 cells) supporting 1024+ dynamic lights (`cluster_cull.wgsl`).
- Cook-Torrance microfacet PBR BRDF (GGX $D$, Smith $V$, Schlick $F$) (`pbr_forward.wgsl`).
- Cascaded Shadow Maps (CSM) with 4 splits, texel snapping, and 3x3 PCF (`shadow_depth.wgsl`).
- Real-Time Global Illumination (RTGI) irradiance probes, Screen-Space Subsurface Scattering (SSS), and Volumetric fog.

### 5.6 Deterministic Physics & Biomechanics (`fluorite_core::physics`)
- Fixed 60Hz deterministic substepping with Continuous Collision Detection (CCD) via `rapier3d`.
- Kinematic Character Controller (KCC) with autostep, slope sliding, and ground snapping.
- 4-wheel vehicle dynamics with Pacejka tire friction model.
- Euphoria-class `BiomechanicalController` for procedural muscle torque and balance.
- Aerohydrodynamics for buoyancy and fluid drag.

### 5.7 Procedural AI & Navigation (`fluorite_core::ai`)
- Batch Line-of-Sight (LOS) raycasting.
- Crowd Density Flow Fields for macroscopic crowd pathfinding.
- Daily NPC schedule state machines.
- Dynamic Police Response Tree with escalating heat levels (1 to 5).

### 5.8 Ray-Traced Spatial Acoustics (`fluorite_core::audio`)
- Ray-traced sound reflection, obstruction, and transmission paths.
- Physical Sabine RT60 reverberation modeling based on acoustic material properties.

### 5.9 World Streaming & Virtual Textures (`fluorite_core::streaming`)
- Asynchronous NVMe/disk chunk streaming manager.
- Sparse page table Virtual Texturing with GPU feedback buffers.

### 5.10 Reticulum Mesh Networking (`fluoderpod_render::reticulum_bridge`)
- Decentralized, offline-first mesh communications over Reticulum Network Stack (RNS).
- Nexus Protocol messages: `Telemetry`, `TensorDiff`, `Ping`.

### 5.11 OpenXR & VR Headset Integration (`fluoderpod_render::xr`)
- Direct OpenXR swapchain rendering via `wgpu_hal` on Android / Meta Quest 3.
- Head and 6-DOF controller tracking synchronized with Rust simulation.

### 5.12 Flame Bridge & 2D/3D Compositing (`fluorescent_flame`)
- `FluorescentViewport` component compositing 3D Vulkan/WebGPU textures directly into Flame canvas.

### 5.13 Creator Tools (`tools/`)
- `asset_pipeline`: glTF 2.0 and WGSL compiler generating binary `.fworld` files.
- `blender_sync`: WebSocket live synchronizer for Blender.
- `higgsfield_bridge`: Generative AI mesh and texture creation.

---

## 6. Verification Test Infrastructure

Fluorescent provides automated test suites across multiple tiers:
- **Tier 1 (`tier1_feature_coverage_test.rs` / `.dart`)**: Comprehensive unit coverage across every subsystem.
- **Tier 2 (`tier2_boundary_corner_test.rs` / `.dart`)**: Memory limits, cache alignments, NaN/Inf floats, and buffer overflows.
- **Tier 3 (`tier3_cross_feature_test.rs` / `.dart`)**: Cross-subsystem integration (Physics + Rendering + BVH).
- **Tier 4 (`tier4_real_world_scenarios_test.rs` / `.dart`)**: 10,000 entity culling benchmarks ($<2\text{ms}$) and 1,000-frame stability stress tests.
- **E2E Suite (`tests/e2e_runner.dart`)**: Complete Flutter / Rust FFI validation.

---
*For complete API signatures and step-by-step code tutorials, see [docs/FEATURES_AND_IMPLEMENTATION_GUIDE.md](../../docs/FEATURES_AND_IMPLEMENTATION_GUIDE.md).*
