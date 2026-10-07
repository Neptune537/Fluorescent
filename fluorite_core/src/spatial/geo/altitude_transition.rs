//! Altitude Boundary Transition Subsystem (~500m).
//!
//! Seamlessly orchestrates the dual-coordinate shift between the global orbital ECEF sphere
//! and the local street-level playing field at the 500-meter altitude threshold.
//! Automatically locks the local origin, calculates camera-relative ENU coordinates
//! in 64-bit precision to eliminate vertex jitter, and activates/deactivates static Rapier3D
//! physics mesh colliders.

use crate::physics::PhysicsWorld;
use crate::spatial::geo::coords::{ecef_to_enu, lat_lon_alt_to_ecef, WGS84_A, WGS84_B};
use glam::Vec3;
use rapier3d::geometry::{ColliderBuilder, ColliderHandle};
use rapier3d::na::Point3;
use serde::{Deserialize, Serialize};

/// Altitude boundary threshold where the engine transitions from orbital to ground mode (meters).
pub const ALTITUDE_TRANSITION_THRESHOLD_METERS: f64 = 500.0;
/// Hysteresis buffer to prevent rapid flipping when hovering near 500m.
pub const ALTITUDE_HYSTERESIS_METERS: f64 = 20.0;

/// Operational mode of the engine view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportScaleMode {
    /// Orbital view: Earth-center origin (0,0,0), 64-bit ECEF coordinates, physics dormant.
    Orbital,
    /// Transitioning between orbital and ground (cross-fading LOD and camera smoothing).
    Transitioning,
    /// Ground playing field: Locked local tile origin, ENU camera-relative, physics active.
    GroundField,
}

/// Transition state and telemetry snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionState {
    pub mode: ViewportScaleMode,
    /// Current observer altitude above WGS84 ellipsoid (meters).
    pub current_altitude: f64,
    /// Transition blend factor: 0.0 (fully orbital) to 1.0 (fully ground).
    pub blend_factor: f32,
    /// Whether the static terrain collision mesh is currently registered in PhysicsWorld.
    pub is_physics_collider_active: bool,
    /// Active local origin in ECEF coordinates (X, Y, Z meters).
    pub local_origin_ecef: [f64; 3],
    /// Active local origin geographic coordinates: (lat_deg, lon_deg, alt_m).
    pub local_origin_geo: (f64, f64, f64),
}

/// Manages the seamless space-to-street boundary transition and physics collider lifecycle.
pub struct AltitudeTransitionManager {
    /// Current mode.
    pub mode: ViewportScaleMode,
    /// Target transition threshold in meters (default: 500m).
    pub threshold_altitude: f64,
    /// Locked ground tile origin in ECEF coordinates.
    pub local_origin_ecef: [f64; 3],
    /// Locked ground tile origin geographic coordinates (lat, lon, alt).
    pub local_origin_geo: (f64, f64, f64),
    /// Active terrain collider handle in the Rapier3D physics world, if resident.
    pub terrain_collider_handle: Option<ColliderHandle>,
    /// Transition smoothing blend factor (0.0 = orbital, 1.0 = ground).
    pub blend_factor: f32,
    /// Transition range for smooth interpolation (e.g. 500m .. 1500m).
    pub transition_range: f64,
}

impl Default for AltitudeTransitionManager {
    fn default() -> Self {
        Self::new(ALTITUDE_TRANSITION_THRESHOLD_METERS)
    }
}

impl AltitudeTransitionManager {
    pub fn new(threshold_altitude: f64) -> Self {
        Self {
            mode: ViewportScaleMode::Orbital,
            threshold_altitude,
            local_origin_ecef: [0.0, 0.0, 0.0],
            local_origin_geo: (0.0, 0.0, 0.0),
            terrain_collider_handle: None,
            blend_factor: 0.0,
            transition_range: 500.0, // Blends between 500m and 1000m
        }
    }

    /// Evaluates camera position, computes altitude above terrain/ellipsoid,
    /// updates transition state, and activates/deactivates physics colliders accordingly.
    pub fn update(
        &mut self,
        camera_lat: f64,
        camera_lon: f64,
        camera_alt: f64,
        physics_world: &mut PhysicsWorld,
        terrain_heightmap_data: Option<(&[f32], usize, usize, f32)>, // (samples, width, height, cell_size_meters)
    ) -> TransitionState {
        let enter_ground_threshold = self.threshold_altitude;
        let exit_ground_threshold = self.threshold_altitude + ALTITUDE_HYSTERESIS_METERS;

        // Calculate transition blend factor (0.0 at upper range, 1.0 at ground threshold)
        let upper_bound = self.threshold_altitude + self.transition_range;
        if camera_alt <= self.threshold_altitude {
            self.blend_factor = 1.0;
        } else if camera_alt >= upper_bound {
            self.blend_factor = 0.0;
        } else {
            let progress = (upper_bound - camera_alt) / self.transition_range;
            self.blend_factor = progress.clamp(0.0, 1.0) as f32;
        }

        match self.mode {
            ViewportScaleMode::Orbital => {
                if camera_alt <= enter_ground_threshold {
                    // Transition to Ground Playing Field
                    self.lock_local_origin(camera_lat, camera_lon, 0.0);
                    self.activate_physics_collider(physics_world, terrain_heightmap_data);
                    self.mode = ViewportScaleMode::GroundField;
                } else if camera_alt < upper_bound {
                    self.mode = ViewportScaleMode::Transitioning;
                }
            }
            ViewportScaleMode::Transitioning => {
                if camera_alt <= enter_ground_threshold {
                    self.lock_local_origin(camera_lat, camera_lon, 0.0);
                    self.activate_physics_collider(physics_world, terrain_heightmap_data);
                    self.mode = ViewportScaleMode::GroundField;
                } else if camera_alt >= upper_bound {
                    self.mode = ViewportScaleMode::Orbital;
                }
            }
            ViewportScaleMode::GroundField => {
                if camera_alt > exit_ground_threshold {
                    // Ascended above boundary: deactivate physics collider and transition to orbital
                    self.deactivate_physics_collider(physics_world);
                    self.mode = if camera_alt < upper_bound {
                        ViewportScaleMode::Transitioning
                    } else {
                        ViewportScaleMode::Orbital
                    };
                }
            }
        }

        TransitionState {
            mode: self.mode,
            current_altitude: camera_alt,
            blend_factor: self.blend_factor,
            is_physics_collider_active: self.terrain_collider_handle.is_some(),
            local_origin_ecef: self.local_origin_ecef,
            local_origin_geo: self.local_origin_geo,
        }
    }

    /// Locks the world origin (0, 0, 0) to a specific ground tile center.
    pub fn lock_local_origin(&mut self, lat_deg: f64, lon_deg: f64, alt_m: f64) {
        self.local_origin_geo = (lat_deg, lon_deg, alt_m);
        let (x, y, z) = lat_lon_alt_to_ecef(lat_deg, lon_deg, alt_m);
        self.local_origin_ecef = [x, y, z];
    }

    /// Computes Camera-Relative position with 64-bit precision subtraction
    /// before downcasting to 32-bit Vec3 to completely eliminate vertex jittering.
    pub fn to_camera_relative(&self, object_ecef: [f64; 3], camera_ecef: [f64; 3]) -> Vec3 {
        let dx = (object_ecef[0] - camera_ecef[0]) as f32;
        let dy = (object_ecef[1] - camera_ecef[1]) as f32;
        let dz = (object_ecef[2] - camera_ecef[2]) as f32;
        Vec3::new(dx, dy, dz)
    }

    /// Converts an absolute ECEF position into local East-North-Up (ENU) coordinates
    /// relative to the locked ground origin.
    pub fn ecef_to_local_enu(&self, object_ecef: [f64; 3]) -> Vec3 {
        let (e, n, u) = ecef_to_enu(
            object_ecef[0],
            object_ecef[1],
            object_ecef[2],
            self.local_origin_geo.0,
            self.local_origin_geo.1,
            self.local_origin_geo.2,
        );
        Vec3::new(e as f32, u as f32, -n as f32) // Map to engine coords: +X=East, +Y=Up, -Z=North
    }

    /// Builds and registers a static terrain physics mesh collider in the Rapier3D physics world.
    fn activate_physics_collider(
        &mut self,
        physics_world: &mut PhysicsWorld,
        heightmap_data: Option<(&[f32], usize, usize, f32)>,
    ) {
        if self.terrain_collider_handle.is_some() {
            return; // Already active
        }

        let collider = if let Some((samples, w, h, cell_size)) = heightmap_data {
            // Build trimesh collider from the heightmap
            let mut vertices = Vec::with_capacity(w * h);
            let half_w = (w as f32 * cell_size) * 0.5;
            let half_h = (h as f32 * cell_size) * 0.5;

            for j in 0..h {
                for i in 0..w {
                    let x = (i as f32 * cell_size) - half_w;
                    let z = (j as f32 * cell_size) - half_h;
                    let y = samples[j * w + i];
                    vertices.push(Point3::new(x, y, z));
                }
            }

            let mut indices = Vec::with_capacity((w - 1) * (h - 1) * 2);
            for j in 0..(h - 1) {
                for i in 0..(w - 1) {
                    let tl = (j * w + i) as u32;
                    let tr = tl + 1;
                    let bl = ((j + 1) * w + i) as u32;
                    let br = bl + 1;

                    indices.push([tl, bl, tr]);
                    indices.push([tr, bl, br]);
                }
            }

            ColliderBuilder::trimesh(vertices, indices).build()
        } else {
            // Fallback ground plane collider (half-space / cuboid at Y = 0)
            ColliderBuilder::cuboid(1000.0, 1.0, 1000.0)
                .translation(rapier3d::na::Vector3::new(0.0, -1.0, 0.0))
                .build()
        };

        let handle = physics_world.insert_standalone_collider(collider);
        self.terrain_collider_handle = Some(handle);
    }

    /// Removes the terrain physics collider from the Rapier3D physics world.
    fn deactivate_physics_collider(&mut self, physics_world: &mut PhysicsWorld) {
        if let Some(handle) = self.terrain_collider_handle.take() {
            physics_world.remove_collider(handle);
        }
    }
}

/// Computes observer altitude above the WGS84 reference ellipsoid from ECEF coordinates.
pub fn ecef_to_altitude(ecef: [f64; 3]) -> f64 {
    let p = (ecef[0] * ecef[0] + ecef[1] * ecef[1]).sqrt();
    let r = (p * p + ecef[2] * ecef[2]).sqrt();
    if r < 1.0 {
        return 0.0;
    }
    // Approximate altitude by subtracting mean earth radius
    let mean_radius = (WGS84_A + WGS84_B) * 0.5;
    r - mean_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_altitude_transition_lifecycle() {
        let mut manager = AltitudeTransitionManager::new(500.0);
        let mut physics = PhysicsWorld::new();

        assert_eq!(manager.mode, ViewportScaleMode::Orbital);
        assert!(manager.terrain_collider_handle.is_none());

        // 1. Camera at 2000m (Orbital)
        let state1 = manager.update(37.7749, -122.4194, 2000.0, &mut physics, None);
        assert_eq!(state1.mode, ViewportScaleMode::Orbital);
        assert_eq!(state1.blend_factor, 0.0);
        assert!(!state1.is_physics_collider_active);
        assert_eq!(physics.num_colliders(), 0);

        // 2. Camera descending into transition zone (750m)
        let state2 = manager.update(37.7749, -122.4194, 750.0, &mut physics, None);
        assert_eq!(state2.mode, ViewportScaleMode::Transitioning);
        assert!(state2.blend_factor > 0.0 && state2.blend_factor < 1.0);
        assert!(!state2.is_physics_collider_active);

        // 3. Camera crossing the 500m boundary down to 300m (GroundField)
        let state3 = manager.update(37.7749, -122.4194, 300.0, &mut physics, None);
        assert_eq!(state3.mode, ViewportScaleMode::GroundField);
        assert_eq!(state3.blend_factor, 1.0);
        assert!(state3.is_physics_collider_active);
        assert_eq!(physics.num_colliders(), 1); // Standalone terrain collider added!

        // 4. Camera ascending slightly to 510m (inside hysteresis band: remains in GroundField)
        let state4 = manager.update(37.7749, -122.4194, 510.0, &mut physics, None);
        assert_eq!(state4.mode, ViewportScaleMode::GroundField);
        assert_eq!(physics.num_colliders(), 1);

        // 5. Camera ascending to 1500m (exits back to Orbital)
        let state5 = manager.update(37.7749, -122.4194, 1500.0, &mut physics, None);
        assert_eq!(state5.mode, ViewportScaleMode::Orbital);
        assert!(!state5.is_physics_collider_active);
        assert_eq!(physics.num_colliders(), 0); // Collider cleanly removed!
    }

    #[test]
    fn test_custom_terrain_heightmap_collider_generation() {
        let mut manager = AltitudeTransitionManager::new(500.0);
        let mut physics = PhysicsWorld::new();

        // 4x4 flat heightmap with cell size 10 meters
        let heightmap = vec![0.0f32; 16];
        let state = manager.update(0.0, 0.0, 100.0, &mut physics, Some((&heightmap, 4, 4, 10.0)));

        assert_eq!(state.mode, ViewportScaleMode::GroundField);
        assert!(state.is_physics_collider_active);
        assert_eq!(physics.num_colliders(), 1);
    }

    #[test]
    fn test_jitter_free_camera_relative_subtraction() {
        let manager = AltitudeTransitionManager::new(500.0);
        // Two positions located ~6,378,000 meters from origin, 5 meters apart
        let cam_ecef = [6_378_137.0001, 0.0, 0.0];
        let obj_ecef = [6_378_142.0001, 2.0, 0.0];

        let rel = manager.to_camera_relative(obj_ecef, cam_ecef);
        assert!((rel.x - 5.0).abs() < 1e-4);
        assert!((rel.y - 2.0).abs() < 1e-4);
        assert!((rel.z - 0.0).abs() < 1e-4);
    }
}
