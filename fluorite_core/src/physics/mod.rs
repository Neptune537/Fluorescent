//! Physics Subsystem: High-performance 3D rigid body dynamics, continuous collision
//! detection (CCD), kinematic character controller (KCC), zero-copy ECS transform sync,
//! Euphoria bio-mechanical ragdoll balance, vehicle dynamics, and hydrodynamics.

pub mod aerohydro;
pub mod biomechanics;
pub mod character_controller;
pub mod destruction;
pub mod sync;
pub mod vehicle;
pub mod world;

pub use aerohydro::{FluidMedium, Hydrodynamics};
pub use biomechanics::{BalanceState, BiomechanicalController, MuscleJointActuator};
pub use character_controller::{CharacterController, CharacterMovementResult};
pub use sync::{
    sync_all_transforms_to_ecs, sync_all_transforms_to_slice, sync_body_transform_to_mat4,
    sync_isometry_to_mat4,
};
pub use vehicle::{PacejkaCoefficients, VehicleDynamics, WheelSuspension};
pub use world::{PhysicsWorld, StepStats};

// Re-export nalgebra via Rapier
pub use rapier3d::na as nalgebra;

// Re-export common Rapier types and builders for consumer convenience
pub use rapier3d::prelude::{
    ActiveCollisionTypes, ActiveEvents, Ball, CCDSolver, Capsule, Collider, ColliderBuilder,
    ColliderHandle, ColliderSet, Cone, Cuboid, Cylinder, ImpulseJointHandle, ImpulseJointSet,
    IntegrationParameters, IslandManager, MultibodyJointHandle, MultibodyJointSet, NarrowPhase,
    PhysicsPipeline, QueryFilter, QueryPipeline, Real, RigidBody, RigidBodyActivation,
    RigidBodyBuilder, RigidBodyHandle, RigidBodySet, RigidBodyType,
};
