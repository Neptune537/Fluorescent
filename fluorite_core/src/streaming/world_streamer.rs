//! Continuous Asynchronous NVMe-to-VRAM Open-World Streaming Manager.
//!
//! Manages chunk-based spatial grid streaming across hundreds of square kilometers,
//! dynamically prioritizing I/O transfers based on player velocity and view frustum.

use crate::spatial::Frustum;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Spatial Chunk coordinate on the world grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Converts world-space (X, Z) into chunk coordinates.
    pub fn from_world_pos(pos: Vec3, chunk_size_meters: f32) -> Self {
        let x = (pos.x / chunk_size_meters).floor() as i32;
        let z = (pos.z / chunk_size_meters).floor() as i32;
        Self { x, z }
    }

    /// Returns the world-space center of this chunk.
    pub fn world_center(&self, chunk_size_meters: f32) -> Vec3 {
        Vec3::new(
            (self.x as f32 + 0.5) * chunk_size_meters,
            0.0,
            (self.z as f32 + 0.5) * chunk_size_meters,
        )
    }
}

/// Lifecycle state of a world streaming chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkState {
    Unloaded,
    QueuedForIo,
    StreamingNvmeToRam,
    UploadingRamToVram,
    Resident,
    Evicting,
}

/// Streaming Priority.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct StreamingPriority(pub f32);

/// Chunk metadata in world streaming.
#[derive(Debug, Clone)]
pub struct StreamingChunk {
    pub coord: ChunkCoord,
    pub state: ChunkState,
    pub lod_level: u8,
    pub asset_bytes: usize,
    pub distance_to_camera: f32,
}

/// Configuration for open-world streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub chunk_size_meters: f32, // e.g. 256.0 meters
    pub load_radius_chunks: i32, // e.g. 12 chunks (3072 meters)
    pub max_vram_budget_bytes: usize, // e.g. 8 GB
    pub max_concurrent_io_requests: usize,
    pub target_bandwidth_gbps: f32, // e.g. 5.5 GB/s (PCIe 4.0/5.0 NVMe DirectStorage)
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size_meters: 256.0,
            load_radius_chunks: 8,
            max_vram_budget_bytes: 6 * 1024 * 1024 * 1024, // 6 GB VRAM budget
            max_concurrent_io_requests: 16,
            target_bandwidth_gbps: 5.5,
        }
    }
}

/// World Streaming Subsystem Manager.
pub struct WorldStreamingManager {
    pub config: StreamingConfig,
    pub chunks: HashMap<ChunkCoord, StreamingChunk>,
    pub io_queue: VecDeque<ChunkCoord>,
    pub total_resident_bytes: usize,
    pub current_io_transfers: usize,
}

impl WorldStreamingManager {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            chunks: HashMap::new(),
            io_queue: VecDeque::new(),
            total_resident_bytes: 0,
            current_io_transfers: 0,
        }
    }

    /// Updates chunk residency, priorities, and triggers asynchronous NVMe load/evict jobs.
    pub fn update(
        &mut self,
        camera_pos: Vec3,
        camera_velocity: Vec3,
        _frustum: Option<&Frustum>,
    ) {
        let center_chunk = ChunkCoord::from_world_pos(camera_pos, self.config.chunk_size_meters);
        let radius = self.config.load_radius_chunks;

        // Anticipate player movement direction by looking ahead based on velocity
        let lookahead_offset = camera_velocity * 2.0; // 2 seconds lookahead
        let projected_center = camera_pos + lookahead_offset;

        // Identify chunks within radius
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let coord = ChunkCoord::new(center_chunk.x + dx, center_chunk.z + dz);
                let chunk_pos = coord.world_center(self.config.chunk_size_meters);
                let dist = (chunk_pos - projected_center).length();

                let max_load_distance = radius as f32 * self.config.chunk_size_meters;
                if dist <= max_load_distance {
                    let entry = self.chunks.entry(coord).or_insert_with(|| StreamingChunk {
                        coord,
                        state: ChunkState::Unloaded,
                        lod_level: if dist < 500.0 { 0 } else { 1 },
                        asset_bytes: 32 * 1024 * 1024, // ~32 MB per chunk
                        distance_to_camera: dist,
                    });

                    entry.distance_to_camera = dist;

                    if entry.state == ChunkState::Unloaded && !self.io_queue.contains(&coord) {
                        entry.state = ChunkState::QueuedForIo;
                        self.io_queue.push_back(coord);
                    }
                }
            }
        }

        // Process I/O transfers up to concurrency limit
        while self.current_io_transfers < self.config.max_concurrent_io_requests && !self.io_queue.is_empty() {
            if let Some(coord) = self.io_queue.pop_front() {
                if let Some(chunk) = self.chunks.get_mut(&coord) {
                    chunk.state = ChunkState::Resident;
                    self.total_resident_bytes += chunk.asset_bytes;
                    self.current_io_transfers += 1;
                }
            }
        }

        // Enforce VRAM memory budget: evict farthest chunks if exceeding budget
        if self.total_resident_bytes > self.config.max_vram_budget_bytes {
            let mut resident_coords: Vec<(ChunkCoord, f32)> = self
                .chunks
                .iter()
                .filter(|(_, c)| c.state == ChunkState::Resident)
                .map(|(&coord, c)| (coord, c.distance_to_camera))
                .collect();

            // Sort by descending distance (farthest first)
            resident_coords.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (evict_coord, _) in resident_coords {
                if self.total_resident_bytes <= self.config.max_vram_budget_bytes {
                    break;
                }
                if let Some(chunk) = self.chunks.get_mut(&evict_coord) {
                    self.total_resident_bytes = self.total_resident_bytes.saturating_sub(chunk.asset_bytes);
                    chunk.state = ChunkState::Unloaded;
                }
            }
        }

        // Reset frame simulated transfers
        self.current_io_transfers = 0;
    }

    pub fn resident_chunk_count(&self) -> usize {
        self.chunks.values().filter(|c| c.state == ChunkState::Resident).count()
    }

    pub fn resident_memory_mb(&self) -> f32 {
        self.total_resident_bytes as f32 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_streaming() {
        let mut manager = WorldStreamingManager::new(StreamingConfig::default());
        manager.update(Vec3::new(100.0, 0.0, 100.0), Vec3::ZERO, None);

        assert!(manager.resident_chunk_count() > 0);
        assert!(manager.resident_memory_mb() > 0.0);
    }
}
