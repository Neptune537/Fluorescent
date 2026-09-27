// # Fluorite Core
//
// High-performance Rust native foundation and custom zero-fragmentation memory allocators
// for the Fluorite AAA Game Engine.
//
// ## Modules
// - `allocator`: Custom memory allocators (`ArenaAllocator`, `DoubleBufferedFrameAllocator`).
// - `api`: Engine lifecycle and memory allocation API for zero-copy FFI bridge.
// - `rendering`: PBR rendering, glTF loader, 3D camera projection, RTGI, SSS, CSM, volumetrics, and Vulkan swapchain surface embedding.
// - `streaming`: Virtual texturing and asynchronous NVMe open-world asset streaming.
// - `physics`: Rapier3D rigid body dynamics, Euphoria bio-mechanical balance, vehicle suspension, and hydrodynamics.
// - `ai`: Line-of-sight checks, crowd flow fields, NPC daily routines, and police response trees.
// - `animation`: Two-bone IK, FABRIK solvers, and mocap blending.
// - `audio`: Real-time acoustic ray-tracing, Sabine RT60 reverb, and dynamic sound occlusion.
// - `fluoropod`: Zero-copy telemetry ring buffers, GC firewall, and input command marshaling.

pub mod ai;
pub mod allocator;
pub mod animation;
pub mod api;
pub mod audio;
pub mod ecs;
pub mod ffi;
pub mod fluoropod;
pub mod frb_generated;
pub mod jni;
pub mod physics;
pub mod rendering;
pub mod servers;
pub mod spatial;
pub mod streaming;

pub use allocator::{
    AllocError, ArenaAllocator, CustomAllocator, DoubleBufferedFrameAllocator, ONE_MB,
    SENTINEL_FOOTER, SENTINEL_HEADER,
};

pub use api::{
    allocate_engine_buffer, get_engine_status, start_engine, verify_buffer_sentinels,
    verify_buffer_sentinels_slice, EngineStatus, EngineStatusC, SharedFrameBuffer,
    DEFAULT_ENGINE_FRAME_CAPACITY,
};

pub use ai::{
    ActivityType, CrowdAgent, CrowdDensityField, DailySchedule, DispatchUnitType, LosBatchQuery,
    LosBatchResult, LosChecker, NpcRoutineState, PoliceHeatLevel, PoliceResponseTree,
};

pub use animation::{solve_fabrik, solve_two_bone_ik, IkJoint, TwoBoneIkResult};

pub use audio::{
    AcousticMaterial, AcousticRay, AcousticResult, AcousticTracer, SoundEmitter, SoundListener,
};

pub use fluoropod::{
    BinarySchemaGenerator, CombatLogEvent, FluoropodSchema, GamepadState, GcFirewall,
    InputCommand, InputCommandQueue, InputEventType, KeyCode, MouseButton, RadarBlip,
    SanitizedUiState, SpscRingBuffer, TelemetryFrame,
};

pub use physics::{
    BalanceState, BiomechanicalController, CharacterController, CharacterMovementResult,
    FluidMedium, Hydrodynamics, MuscleJointActuator, PacejkaCoefficients, PhysicsWorld,
    StepStats, VehicleDynamics, WheelSuspension,
};

pub use rendering::{
    Camera3D, CascadeData, CascadedShadowMap, CsmConfig, GltfMeshLoader, IrradianceProbe,
    Model3D, ProjectionType, QualityTier, Renderer, RtgiConfig, RtgiPipeline, SubMesh,
    SubsurfaceProfile, SurfaceConfig, SurfaceFormat, SwapchainStatus, TlasInstance, Vertex3D,
    VolumetricAtmosphereConfig, VulkanSurfaceEmbedder, WaterCausticsConfig, WindowHandleType,
};

pub use spatial::{
    Aabb, FlatBvh, FlatBvhNode, Frustum, FrustumCullingStats, FrustumIntersection, Plane, Ray,
    RayHit,
};

pub use streaming::{
    ChunkCoord, ChunkState, FeedbackBuffer, PageCoord, PageTableEntry, StreamingChunk,
    StreamingConfig, StreamingPriority, VirtualTextureSystem, WorldStreamingManager,
};
