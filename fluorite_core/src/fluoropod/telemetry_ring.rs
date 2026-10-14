//! Lock-Free SPSC Shared Memory Ring Buffer for High-Rate Telemetry Streaming.
//!
//! Provides atomic, zero-contention streaming of player stats, radar blips, and combat events
//! at 60/120/144 FPS from Rust native threads to the Flutter UI isolate without memory allocations.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const TELEMETRY_RING_CAPACITY: usize = 256;
pub const MAX_RADAR_BLIPS: usize = 32;
pub const MAX_ACTIVE_LINKS: usize = 16;
pub const MAX_EDGE_NODES: usize = 32;

pub const NODE_ROLE_MASTER: u8 = 0;
pub const NODE_ROLE_EDGE_WORKER: u8 = 1;

/// Proof-of-Health metrics for edge compute nodes (Bioluminescence Telemetry)
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct ProofOfHealth {
    pub thermal_celsius: f32,
    pub vram_allocated_pct: f32,
    pub battery_pct: f32,
    pub _pad: u32,
}

impl Default for ProofOfHealth {
    fn default() -> Self {
        Self {
            thermal_celsius: 0.0,
            vram_allocated_pct: 0.0,
            battery_pct: 0.0,
            _pad: 0,
        }
    }
}

/// Compute throughput metrics for edge compute pipelines
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct ComputeThroughput {
    pub tokens_per_sec: f32,
    pub active_layers_assigned: u8,
    pub _pad0: [u8; 3], // padding for u64 alignment
    pub kv_cache_bytes: u64,
}

impl Default for ComputeThroughput {
    fn default() -> Self {
        Self {
            tokens_per_sec: 0.0,
            active_layers_assigned: 0,
            _pad0: [0; 3],
            kv_cache_bytes: 0,
        }
    }
}

/// Telemetry payload for a distributed edge AI compute node
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct EdgeNodeTelemetry {
    pub node_id: u32,
    pub node_role: u8,
    pub _pad0: [u8; 3], // align to 8 bytes for f64
    #[serde(with = "serde_arrays")]
    pub did: [u8; 64],  // did:nexus: string identifier
    pub ecef_x: f64,
    pub ecef_y: f64,
    pub ecef_z: f64,
    pub proof_of_health: ProofOfHealth,
    pub compute_throughput: ComputeThroughput,
    pub active_link_count: u32,
    pub active_links: [u32; MAX_ACTIVE_LINKS],
    pub _pad1: u32, // align to 8 bytes size boundary
}

impl Default for EdgeNodeTelemetry {
    fn default() -> Self {
        Self {
            node_id: 0,
            node_role: NODE_ROLE_MASTER,
            _pad0: [0; 3],
            did: [0; 64],
            ecef_x: 0.0,
            ecef_y: 0.0,
            ecef_z: 0.0,
            proof_of_health: ProofOfHealth::default(),
            compute_throughput: ComputeThroughput::default(),
            active_link_count: 0,
            active_links: [0; MAX_ACTIVE_LINKS],
            _pad1: 0,
        }
    }
}

/// Compact radar blip representation for tactical HUD overlay.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct RadarBlip {
    pub entity_id: u32,
    pub relative_x: f32, // Distance along X axis in meters
    pub relative_z: f32, // Distance along Z axis in meters
    pub altitude_offset: f32,
    pub is_hostile: u8,
    pub blip_type: u8, // 0 = infantry, 1 = vehicle, 2 = objective, 3 = projectile
    pub _pad: [u8; 2],
}

impl Default for RadarBlip {
    fn default() -> Self {
        Self {
            entity_id: 0,
            relative_x: 0.0,
            relative_z: 0.0,
            altitude_offset: 0.0,
            is_hostile: 0,
            blip_type: 0,
            _pad: [0; 2],
        }
    }
}

/// Combat log event for HUD kill feed and status messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLogEvent {
    pub timestamp_ms: u64,
    pub source_name: String,
    pub target_name: String,
    pub damage_amount: f32,
    pub is_critical: bool,
}

/// Telemetry frame payload emitted per simulation tick.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct TelemetryFrame {
    pub frame_index: u64,
    pub timestamp_us: u64,
    pub player_health: f32,
    pub player_max_health: f32,
    pub player_shield: f32,
    pub player_max_shield: f32,
    pub player_energy: f32,
    pub player_max_energy: f32,
    pub heat_level: u32,
    pub blip_count: u32,
    pub radar_blips: [RadarBlip; MAX_RADAR_BLIPS],
    pub edge_node_count: u32,
    pub _pad: u32,
    pub edge_nodes: [EdgeNodeTelemetry; MAX_EDGE_NODES],
}

impl Default for TelemetryFrame {
    fn default() -> Self {
        Self {
            frame_index: 0,
            timestamp_us: 0,
            player_health: 100.0,
            player_max_health: 100.0,
            player_shield: 100.0,
            player_max_shield: 100.0,
            player_energy: 100.0,
            player_max_energy: 100.0,
            heat_level: 0,
            blip_count: 0,
            radar_blips: [RadarBlip::default(); MAX_RADAR_BLIPS],
            edge_node_count: 0,
            _pad: 0,
            edge_nodes: [EdgeNodeTelemetry::default(); MAX_EDGE_NODES],
        }
    }
}

/// Single-Producer Single-Consumer (SPSC) Circular Ring Buffer.
///
/// Head and tail indices are cache-line aligned (64 bytes) to prevent CPU false sharing.
pub struct SpscRingBuffer<T: Copy + Default, const N: usize> {
    buffer: [T; N],
    _pad0: [u8; 64],
    head: AtomicUsize, // Written by Producer, read by Consumer
    _pad1: [u8; 64],
    tail: AtomicUsize, // Written by Consumer, read by Producer
    _pad2: [u8; 64],
}

impl<T: Copy + Default, const N: usize> SpscRingBuffer<T, N> {
    pub fn new() -> Self {
        assert!(N.is_power_of_two(), "Capacity N must be a power of two");
        Self {
            buffer: [T::default(); N],
            _pad0: [0; 64],
            head: AtomicUsize::new(0),
            _pad1: [0; 64],
            tail: AtomicUsize::new(0),
            _pad2: [0; 64],
        }
    }

    /// Pushes a new item onto the ring buffer (called by native Rust simulation thread).
    ///
    /// If full, overwrites the oldest element to guarantee the UI always receives freshest data.
    pub fn push(&mut self, item: T) {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        let next_head = (head + 1) & (N - 1);

        if next_head == tail {
            // Buffer full: advance tail to discard oldest unconsumed frame
            self.tail.store((tail + 1) & (N - 1), Ordering::Release);
        }

        self.buffer[head] = item;
        self.head.store(next_head, Ordering::Release);
    }

    /// Pops the latest item from the ring buffer (called by Dart FFI polling or frame callback).
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if head == tail {
            None // Empty
        } else {
            let item = self.buffer[tail];
            self.tail.store((tail + 1) & (N - 1), Ordering::Release);
            Some(item)
        }
    }

    /// Returns the number of unread frames in the buffer.
    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        (head + N - tail) & (N - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spsc_ring_buffer() {
        let mut ring: SpscRingBuffer<u32, 8> = SpscRingBuffer::new();
        assert_eq!(ring.pop(), None);

        ring.push(42);
        ring.push(100);
        assert_eq!(ring.available(), 2);

        assert_eq!(ring.pop(), Some(42));
        assert_eq!(ring.pop(), Some(100));
        assert_eq!(ring.pop(), None);
    }
}
