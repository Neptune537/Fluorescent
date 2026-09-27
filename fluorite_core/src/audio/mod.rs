//! Real-Time Acoustic Ray-Tracing and Dynamic Spatial Reverb Subsystem.
//!
//! Traces audio rays through 3D geometry to determine early reflections,
//! Sabine/Eyring $RT_{60}$ late reverberation decay, and geometric sound occlusion.

pub mod acoustics;

pub use acoustics::{
    AcousticMaterial, AcousticRay, AcousticResult, AcousticTracer, SoundEmitter, SoundListener,
};
