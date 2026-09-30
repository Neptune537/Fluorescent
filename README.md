<p align="center">
  <h1 align="center">🔬 Fluorescent Engine</h1>
  <p align="center">
    <strong>AAA • Open Source • Cross-Platform</strong>
  </p>
  <p align="center">
    Flutter for the interface. Rust for the heart. Vulkan & WebGPU for the hardware.
  </p>
  <p align="center">
    <a href="docs/FEATURES_AND_IMPLEMENTATION_GUIDE.md"><img src="https://img.shields.io/badge/docs-feature_guide-blueviolet?style=flat-square" alt="Feature Guide"></a>
    <a href="docs/ENGINE_SPECIFICATION.md"><img src="https://img.shields.io/badge/docs-engine_spec-blue?style=flat-square" alt="Engine Spec"></a>
    <img src="https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square&logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/flutter-3.22%2B-02569B?style=flat-square&logo=flutter" alt="Flutter">
    <img src="https://img.shields.io/badge/vulkan-1.3-red?style=flat-square&logo=vulkan" alt="Vulkan">
    <img src="https://img.shields.io/badge/webgpu-wgpu-green?style=flat-square" alt="WebGPU">
    <img src="https://img.shields.io/badge/license-Apache_2.0-green?style=flat-square" alt="License">
    <img src="https://img.shields.io/badge/status-active_development-yellow?style=flat-square" alt="Status">
  </p>
</p>

---

Fluorescent is a **layered game engine and runtime platform** that combines a native Rust core with Flutter's UI framework and a high-throughput GPU-driven Vulkan / WebGPU rendering pipeline.

```
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

## ✨ Why Fluorescent?

| What | How |
|------|-----|
| **Rust is the heart** | All gameplay state, physics, networking, and rendering commands are owned by Rust. Memory-safe, data-race-free, zero-GC. |
| **Flutter is the interface** | The editor, HUDs, menus, inventory, chat, and all game UI are built with Flutter — the best cross-platform UI framework. |
| **GPU-Driven Rendering** | Frustum culling, HZB occlusion, and Nanite-style cluster LOD evaluation happen entirely in compute shaders on the GPU. |
| **Two engines in one** | A **Creation Runtime** (Flutter editor) and a **Game Runtime** (Rust core) share the same project. |
| **Mobile-First Thermal Scaling** | Dynamic frame rate throttling drops power consumption by up to 75% when scenes are static or idle. |
| **Zero-copy FFI** | Rust and Flutter share memory directly via 64-byte aligned arenas, `PackedEntityInstance` structs, and native GPU texture IDs. |

---

## 🎯 Target Platforms

> [!IMPORTANT]
> **Strict Platform Focus: Android and Web Only**
> - **Android (Vulkan 1.3 / NDK):** Zero-copy `AHardwareBuffer` / `ANativeWindow` texture registry bridge, OpenXR VR support (Meta Quest 3).
> - **Web (WebGPU / Wasm):** `wasm32-unknown-unknown` pipeline rendering directly into Flutter Web canvas.
> - **Desktop & Server:** Dedicated developer tooling (`fluorite_editor`) and headless server streaming via WebRTC.

---

## 📦 Repository Structure

```
Fluorescent/
├── docs/                               # Formal specifications & manuals
│   ├── FEATURES_AND_IMPLEMENTATION_GUIDE.md  # Complete features manual & how-to guides
│   ├── ENGINE_SPECIFICATION.md         # Master architectural spec
│   ├── GPU_DRIVEN_ARCHITECTURE.md      # Compute culling & virtual geometry design
│   ├── OPENXR_INTEGRATION.md           # OpenXR & Meta Quest 3 VR pipeline
│   └── RETICULUM_BRIDGE.md             # Reticulum mesh networking bridge
│
├── fluorite_core/                      # Rust runtime core
│   ├── src/
│   │   ├── allocator/                  # Arena & Double-buffered frame allocators
│   │   ├── ecs/                        # High-performance sparse-set ECS
│   │   ├── spatial/                    # 32-byte FlatBvhNode, 16-bin SAH BVH, SIMD culling
│   │   ├── rendering/                  # PBR, Clustered Forward+, CSM, RTGI, SSS, Volumetrics
│   │   ├── physics/                    # Rapier3D, Character Controller, Vehicles, Biomechanics
│   │   ├── ai/                         # Line-of-sight, Crowd flow fields, Police response trees
│   │   ├── audio/                      # Ray-traced acoustics, Sabine RT60 reverb
│   │   ├── streaming/                  # World chunk streamer, Virtual texturing
│   │   └── fluoropod/                  # GC firewall, SPSC ring buffer, telemetry frames
│   └── Cargo.toml
│
├── fluoderpod_render/                  # GPU-driven rendering pipeline
│   ├── src/
│   │   ├── culling/                    # Compute Frustum & HZB Occlusion compute shaders
│   │   ├── virtual_geometry/           # Nanite-style cluster LOD evaluation & streaming
│   │   ├── unified_pipeline/           # Indirect draw buffers & wgpu device management
│   │   ├── thermal/                    # Dynamic mobile thermal & power scaler
│   │   ├── android_vulkan.rs           # Zero-copy Android AHardwareBuffer Vulkan bridge
│   │   ├── reticulum_bridge.rs         # Reticulum Network Stack (RNS) mesh bridge
│   │   ├── xr.rs                       # OpenXR / Meta Quest 3 VR integration
│   │   └── webrtc_streamer.rs          # WebRTC video streamer & signalling
│   └── Cargo.toml
│
├── fluorescent/                        # Dart / Flutter packages monorepo
│   └── packages/
│       ├── fluorescent_core/           # Render graph, scene graph, resource management
│       ├── fluorescent_ecs/            # TypedData Float32List ECS
│       ├── fluorescent_flame/          # Flame integration (FluorescentViewport)
│       ├── fluorescent_vulkan/         # Android Vulkan texture bridge
│       ├── fluorescent_webgpu/         # Web WebGPU canvas binding
│       └── fluorescent_fluorite/       # Automotive AAOS adapter
│
├── fluorite_editor/                    # Flutter desktop developer tooling & inspector
├── tools/
│   ├── asset_pipeline/                 # CLI: .gltf/.wgsl → .fworld binary compiler
│   ├── shader_compiler/                # WGSL shader generation & validation
│   ├── blender_sync/                   # Live Blender ↔ Engine WebSocket sync add-on
│   └── higgsfield_bridge/              # Generative AI 3D mesh & texture generator
│
├── examples/
│   ├── hybrid_2d_3d/                   # Flame 2D + Fluorescent 3D compositing sample
│   └── open_world_demo/                # Streaming open world demo
│
└── tests/                              # Multi-tier verification test suite
    ├── tier1_feature_coverage_test.*   # Subsystem unit coverage
    ├── tier2_boundary_corner_test.*    # Memory & boundary test suite
    ├── tier3_cross_feature_test.*      # Physics + Rendering + BVH integration
    └── tier4_real_world_scenarios_test.* # 10k entity culling & 1,000 frame stability
```

---

## 🏗️ 10 Engine Subsystems

| # | Subsystem | Description | Implementation Status |
|---|-----------|-------------|-----------------------|
| 1 | **Core Memory & Allocators** | 16MB double-buffered frame arenas, 64-byte cache alignment, sentinel protection, GC firewall | ✅ Fully Implemented |
| 2 | **ECS & Spatial Indexing** | Sparse-set contiguous transforms, 32-byte FlatBVH, 16-bin SAH builder (<0.2ms for 10k entities) | ✅ Fully Implemented |
| 3 | **GPU-Driven Rendering** | Compute frustum culling, HZB occlusion, Nanite-style cluster LOD, unified indirect draws | ✅ Fully Implemented |
| 4 | **PBR & Lighting** | Clustered Forward+ (3,456 cells, 1024+ lights), Cook-Torrance GGX, 4-split CSM, RTGI, SSS | ✅ Fully Implemented |
| 5 | **Physics & Simulation** | Rapier3D 60Hz deterministic physics, KCC character controller, Pacejka vehicles, Euphoria biomechanics | ✅ Fully Implemented |
| 6 | **Procedural AI** | Batch Line-of-Sight, Crowd Density Flow Fields, NPC Daily Schedules, Police Response Tree | ✅ Fully Implemented |
| 7 | **Spatial Acoustics** | Ray-traced sound paths, Sabine RT60 reverberation, dynamic material sound occlusion | ✅ Fully Implemented |
| 8 | **Streaming & Virtual Textures**| Asynchronous chunk streaming, sparse page table virtual texturing with GPU feedback | ✅ Fully Implemented |
| 9 | **Networking & Mesh** | Reticulum Network Stack (RNS) mesh bridge, Nexus Protocol (Telemetry, TensorDiff) | ✅ Fully Implemented |
| 10 | **Creator Tools & Pipeline** | `.fworld` glTF compiler, Blender WebSocket live sync, Higgsfield generative AI bridge | ✅ Fully Implemented |

---

## 🎮 Hardware Capability Tiers

The engine dynamically scales based on detected hardware capabilities:

| Tier | Target Hardware | Rendering Pipeline | Shadowing / GI | Virtual Geometry |
|---|---|---|---|---|
| **Tier 1** | Mobile / Web | Forward+ Clustered | 2-split CSM, Simple GI | LOD 0-2 (Traditional) |
| **Tier 2** | Upper Mobile / Desktop | Forward+ / Deferred | 4-split CSM, RTGI Probes | Nanite Cluster Evaluation |
| **Tier 3** | High-End / XR | GPU-Driven Indirect | Full CSM, RTGI, SSS, Volumetrics | Continuous Nanite Streaming |

---

## 🚀 Quick Start

### 1. Run a Hybrid 2D/3D Flame Game
```bash
cd examples/hybrid_2d_3d
flutter run
```

### 2. Ingest High-Throughput 48-Byte Entity Batches
```rust
use fluoderpod_render::unified_pipeline::PackedEntityInstance;

let batch = vec![PackedEntityInstance {
    position: [0.0, 1.0, 0.0],
    sphere_radius: 1.0,
    rotation_quat: [0.0, 0.0, 0.0, 1.0],
    scale: [1.0, 1.0, 1.0],
    cluster_and_flags: 1,
}];

let bytes = bytemuck::cast_slice(&batch);
renderer.ingest_fluoderpod_batch(bytes).unwrap();
```

### 3. Compile 3D Assets to `.fworld`
```bash
cd fluorescent/tools/asset_pipeline
dart run bin/asset_pipeline.dart --gltf character.gltf --shader pbr.wgsl --output character.fworld
```

---

## 📖 Complete Documentation

| Document | Description |
|---|---|
| **[FEATURES_AND_IMPLEMENTATION_GUIDE.md](docs/FEATURES_AND_IMPLEMENTATION_GUIDE.md)** | **Master Guide:** Detailed architecture, API signatures, and 11 step-by-step implementation tutorials |
| **[ENGINE_SPECIFICATION.md](docs/ENGINE_SPECIFICATION.md)** | Formal engine specification, platform architecture, and subsystem boundaries |
| **[GPU_DRIVEN_ARCHITECTURE.md](docs/GPU_DRIVEN_ARCHITECTURE.md)** | Deep dive into Compute Culling, HZB Occlusion, and Virtual Geometry |
| **[OPENXR_INTEGRATION.md](docs/OPENXR_INTEGRATION.md)** | Meta Quest 3 VR rendering pipeline and frame pacing |
| **[RETICULUM_BRIDGE.md](docs/RETICULUM_BRIDGE.md)** | Reticulum Network Stack (RNS) mesh bridge & Nexus protocol |
| **[PROJECT.md](PROJECT.md)** | Master project milestone tracker and feature inventory |
| **[AGENTS.md](AGENTS.md)** | Specialized multi-agent roles for concurrent engineering |

---

## 🧪 Verification & Testing

```bash
# Run Tier 1 subsystem unit coverage
cargo test --test tier1_feature_coverage_test

# Run Tier 4 10k entity culling & stability tests
cargo test --test tier4_real_world_scenarios_test

# Run Flutter / Flame E2E integration suite
dart tests/e2e_runner.dart
```

---

## 📄 License

Licensed under the Apache License 2.0 — see [LICENSE](LICENSE) for details.
