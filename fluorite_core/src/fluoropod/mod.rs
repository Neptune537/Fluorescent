//! Fluoropod: Zero-Copy Bridge & State Synchronization Layer.
//!
//! Connects high-frequency native Rust simulation to the Flutter presentation engine
//! with lock-free telemetry ring buffers, GC firewalls, and sub-millisecond input marshaling.

pub mod fluoropod_gen;
pub mod gc_firewall;
pub mod input_marshaler;
pub mod telemetry_ring;

pub use fluoropod_gen::{BinarySchemaGenerator, FluoropodSchema};
pub use gc_firewall::{GcFirewall, SanitizedUiState};
pub use input_marshaler::{
    GamepadState, InputCommand, InputCommandQueue, InputEventType, KeyCode, MouseButton,
};
pub use telemetry_ring::{
    CombatLogEvent, RadarBlip, SpscRingBuffer, TelemetryFrame, TELEMETRY_RING_CAPACITY,
};
