# Fluorite AAA Engine - Tasks

## Phase 2: Functional Test App & Geofencing

### 1. Functional Test App Setup
- Create a new Flutter application (`functional_test_app`). ✅ Completed
- Set up `google_maps_flutter` for the base map rendering. ✅ Completed (Replaced by internal Flame 3D Map)
- Integrate the `fluorite_core` FFI bridge to initialize the native engine. ✅ Completed

### 2. Live Social Map & Geofencing
- Implement a custom map tile provider for Slippy Map tiles. 🚧 Partially Completed
- Add real-time driver location markers. 🚧 Partially Completed
- Implement the Geofencing visual layer (event zones, speed traps). ❌ Missing (Scaffolding required in `functional_test_app`)
- Connect map enter/exit events to the Rust `NavigationServer` or `PhysicsServer` for logical processing. ❌ Missing

### 3. Background Location Tracking
- Integrate a background location tracking package (e.g., `flutter_background_geolocation` or a custom platform channel implementation). ❌ Missing (Scaffolding required)
- Ensure battery-efficient GPS sampling (adaptive update rate). ❌ Missing

### 4. Multiplayer Sync (Rust Core)
- Implement a WebSocket or WebRTC data-channel server within the Rust networking foundation. ✅ Completed (Reticulum Mesh)
- Implement client-side position interpolation and extrapolation (to handle 30 Hz ping limits). 🚧 Skeleton only (`NetworkInterpolationComponent` in `replication.rs`)
- Build the entity culling logic to only render/process drivers within a visible radius. 🚧 Skeleton only (Local player pos retrieved, but actual server-side culling is missing in `replication.rs`)

### 5. Social Feed
- Build the Flutter UI layer for a social feed (club posts, recent activities). ✅ Completed
- Build basic DMs/group channel UI. ✅ Completed
- (Backend integration points left open for Firebase/Stream integration later). ✅ Completed

## Phase 3: AAA Engine Features

### 1. GPU-Driven Rendering & Virtual Geometry
- Implement compute shader-based culling (frustum, occlusion). ✅ Completed
- Develop virtual geometry system (Nanite-style micro-polygon rendering). ✅ Completed
- Transition to unified GPU command buffers to minimize CPU submission overhead. ✅ Completed

### 2. Advanced Lighting (Dynamic GI & Virtual Shadows)
- Implement screen-space or hardware-accelerated raytraced Dynamic Global Illumination. 🚧 Skeleton only (`rtgi.rs`)
- Build Virtual Shadow Maps (VSM) for high-resolution, scalable shadow rendering. 🚧 Skeleton only (`vsm.rs`)
- Integrate temporal upscaling techniques (TAA, FSR2) into the render graph. 🚧 Skeleton only (`dlss.rs`)

### 3. GPU VFX & Particles
- Build a node-based GPU particle simulation framework. ✅ Engine side Completed
- Enable high-count particle rendering (Niagara-style) interacting with the depth/GBuffer. ✅ Engine side Completed
- Implement Editor UI serialization binding for Particle/VFX nodes. ❌ Missing (UI mocks exist, no functional bindings)

### 4. Advanced Animation & Destruction
- Implement runtime bone/skeletal IK constraints and state machines in the Rust engine. 🚧 Skeleton only (`ik.rs`)
- Integrate physics-driven destruction (Chaos-style chunk separation and rigid body spawning). ❌ Missing (`destruction/mod.rs` is empty)

### 5. World Streaming & PCG
- Implement spatial hashing and chunked world streaming for open-world scales. ✅ Completed
- Develop Procedural Content Generation (PCG) frameworks for terrain generation, vegetation placement, and biome mapping. 🚧 Skeleton only (`pcg/mod.rs`)
