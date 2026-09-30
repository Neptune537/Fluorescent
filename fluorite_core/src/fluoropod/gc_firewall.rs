//! Garbage Collector (GC) Firewall.
//!
//! Enforces strict isolation between Rust simulation memory and Flutter's Dart VM heap:
//! - ZERO 3D transform matrices (Mat4) or internal ECS entity handles cross into Dart.
//! - Transforms remain strictly in GPU uniform buffers / native memory buffers.
//! - Only sanitized, aggregate UI state primitives pass through to Dart, guaranteeing
//!   zero Dart GC allocations and stutter-free 60/120 FPS rendering.

use serde::{Deserialize, Serialize};

/// Sanitized UI state exposed to Flutter widgets.
///
/// Contains ONLY display-ready aggregates (e.g. ammo counts, compass heading, speed in km/h),
/// with no scene graphs, meshes, or entity pointers.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct SanitizedUiState {
    pub compass_heading_degrees: f32,
    pub speed_kmh: f32,
    pub altitude_meters: f32,
    pub current_ammo: u32,
    pub reserve_ammo: u32,
    pub weapon_heat_percentage: f32,
    pub target_locked: u32,
    pub target_lead_x: f32, // Screen-space normalized offset (-1.0 to 1.0)
    pub target_lead_y: f32,
    pub active_quest_distance: f32,
}

impl Default for SanitizedUiState {
    fn default() -> Self {
        Self {
            compass_heading_degrees: 0.0,
            speed_kmh: 0.0,
            altitude_meters: 0.0,
            current_ammo: 30,
            reserve_ammo: 120,
            weapon_heat_percentage: 0.0,
            target_locked: 0,
            target_lead_x: 0.0,
            target_lead_y: 0.0,
            active_quest_distance: 150.0,
        }
    }
}

pub struct GcFirewall;

impl GcFirewall {
    /// Sanitizes raw native engine state into GC-safe UI state.
    pub fn sanitize(
        forward_vector: [f32; 3],
        velocity_vector: [f32; 3],
        player_position: [f32; 3],
        current_ammo: u32,
        reserve_ammo: u32,
        weapon_heat: f32,
        target_info: Option<([f32; 2], bool)>,
    ) -> SanitizedUiState {
        // Compute compass heading in degrees [0, 360)
        let heading_rad = forward_vector[0].atan2(forward_vector[2]);
        let mut heading_deg = heading_rad.to_degrees();
        if heading_deg < 0.0 {
            heading_deg += 360.0;
        }

        // Speed in km/h
        let speed_ms = (velocity_vector[0] * velocity_vector[0]
            + velocity_vector[1] * velocity_vector[1]
            + velocity_vector[2] * velocity_vector[2])
            .sqrt();
        let speed_kmh = speed_ms * 3.6;

        let (lead, locked) = match target_info {
            Some((lead_xy, is_locked)) => (lead_xy, if is_locked { 1u32 } else { 0u32 }),
            None => ([0.0, 0.0], 0u32),
        };

        SanitizedUiState {
            compass_heading_degrees: heading_deg,
            speed_kmh,
            altitude_meters: player_position[1],
            current_ammo,
            reserve_ammo,
            weapon_heat_percentage: weapon_heat.clamp(0.0, 1.0),
            target_locked: locked,
            target_lead_x: lead[0],
            target_lead_y: lead[1],
            active_quest_distance: 250.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_firewall_sanitization() {
        let state = GcFirewall::sanitize(
            [0.0, 0.0, 1.0],  // Facing forward (North)
            [10.0, 0.0, 0.0], // 10 m/s = 36 km/h
            [0.0, 15.0, 0.0],
            30,
            90,
            0.2,
            Some(([0.1, -0.2], true)),
        );

        assert_eq!(state.speed_kmh.round(), 36.0);
        assert_eq!(state.target_locked, 1);
        assert_eq!(state.altitude_meters, 15.0);
    }
}
