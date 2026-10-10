# Fluorescent Engine - Remaining Work Multi-Agent Execution Plan

This document outlines the exact architectural plan and scaffolding for the remaining missing or partially completed features across the Fluorescent Engine ecosystem.

Each agent is assigned a specific number, a role, and a set of explicit file paths where scaffolding has been established. The overall goal is to unify the knowledge of remaining work and guide agents directly to the files requiring implementation.

---

## Agent 1: Rendering Architect (Advanced Lighting & Upscaling)
**Objective:** Finalize Phase 3 Advanced Lighting (RTGI, VSM) and Temporal Upscaling.
**Current State:** The architectures for RTGI, VSM, and Upscaling exist as skeletons.
**Scaffolding Locations:**
- `fluoderpod_render/src/advanced_lighting/vsm.rs`: Implement the `execute` method for allocating physical pages, updating page tables, and dispatching shadow rendering compute shaders based on light frustums.
- `fluorite_core/src/rendering/rtgi.rs`: Implement the dispatch logic to trace rays against the `TlasInstance` buffer and reconstruct spherical harmonics for `IrradianceProbe` caching.
- `fluorite_core/src/rendering/dlss.rs`: Fill in the `execute_temporal_upscaling` function blocks for TAA and FSR2 to bind the history buffers and dispatch the respective compute shaders.

---

## Agent 2: Systems & Gameplay Programmer (Destruction & Animation)
**Objective:** Implement missing AAA Animation (IK) and Physics Destruction.
**Current State:** Skeleton structs exist for IK, and an empty module exists for destruction.
**Scaffolding Locations:**
- `fluorite_core/src/animation/ik.rs`: Implement the math for the `solve_two_bone_ik` (analytical cosine rule) and `solve_fabrik` (iterative forward/backward reaching) algorithms.
- `fluorite_core/src/physics/destruction/mod.rs`: Implement the `DestructionSystem` to process mesh fracture events, decompose compound shapes into Voronoi cells, and spawn dynamic Rapier3D rigid bodies.

---

## Agent 3: World Streaming & PCG Engineer
**Objective:** Implement Procedural Content Generation (PCG) for the Open World.
**Current State:** World streaming is implemented, but chunk data generation is empty.
**Scaffolding Locations:**
- `fluorite_core/src/streaming/pcg/mod.rs`: Implement the `generate_chunk` method using 2D/3D Perlin/Simplex noise to populate heightmaps and spawn vegetation entities via the ECS based on the provided `PcgConfig`.

---

## Agent 4: Networking & Cloud Architect
**Objective:** Finalize Phase 2 Multiplayer Synchronization.
**Current State:** Reticulum Mesh runs, but interpolation and server-side culling are just skeletons.
**Scaffolding Locations:**
- `fluorite_core/src/net/replication.rs` (Interpolation): In the rendering extraction phase, calculate the extrapolated `Transform` based on velocity and latency, and apply it to the `TransformComponent` for smooth remote player movement.
- `fluorite_core/src/net/replication.rs` (Culling): Implement the logic in `get_visible_entities` to calculate distances between the local player and remote entities, omitting network replication updates for entities outside the cull radius.

---

## Agent 5: UI/UX & Mobile Integrator
**Objective:** Complete the missing Phase 2 mobile app features (Geofencing & Background Location).
**Current State:** Flutter UI exists, but map integrations for geofencing and background tracking are missing.
**Scaffolding Locations:**
- `fluorescent/examples/functional_test_app/lib/geofencing.dart`: Implement the UI overlay to render event zones and speed traps, and integrate the boundary crossing logic to call the `FluoriteEngineBridge`.
- `fluorescent/examples/functional_test_app/lib/background_location.dart`: Implement a background service (via platform channels or an external package) to sample GPS coordinates and push them to `FluoriteEngineBridge().sendTelemetry()` even when the app is backgrounded.

---

## Agent 6: Tools & Editor Engineer
**Objective:** Connect the Editor UI to the Engine Data Graph.
**Current State:** The Editor has UI nodes for PCG and VFX, but serialization bindings are missing.
**Scaffolding Locations:**
- `fluorite_editor/lib/src/node_editor/serialization.dart`: Implement the missing JSON payload mappings to actually serialize the visual `MobileOptimizedTerrainPCG` and `HalfPrecisionVFX` nodes into engine-compatible Render Graph definitions.
