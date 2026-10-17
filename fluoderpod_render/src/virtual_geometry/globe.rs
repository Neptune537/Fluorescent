use std::collections::HashMap;

pub const EARTH_RADIUS_WGS84: f64 = 6378137.0; // WGS84 semi-major axis (meters)

/// A simple WGS84 tile coordinate structure (for legacy 2D/Mercator lookups)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TileIndex {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

/// The 6 faces of a normalized cube map mapped to a sphere
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CubeFace {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
}

impl CubeFace {
    pub const ALL: [CubeFace; 6] = [
        CubeFace::PositiveX,
        CubeFace::NegativeX,
        CubeFace::PositiveY,
        CubeFace::NegativeY,
        CubeFace::PositiveZ,
        CubeFace::NegativeZ,
    ];

    /// Converts normalized 2D face coordinates (u, v in [-1.0, 1.0]) to a 3D unit cube vector
    #[inline]
    pub fn to_cube_vector(&self, u: f64, v: f64) -> [f64; 3] {
        match self {
            CubeFace::PositiveX => [1.0, -v, -u],
            CubeFace::NegativeX => [-1.0, -v, u],
            CubeFace::PositiveY => [u, 1.0, v],
            CubeFace::NegativeY => [u, -1.0, -v],
            CubeFace::PositiveZ => [u, -v, 1.0],
            CubeFace::NegativeZ => [-u, -v, -1.0],
        }
    }
}

/// A discrete patch in the geocentric quadtree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PatchKey {
    pub face: CubeFace,
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

impl PatchKey {
    pub fn root(face: CubeFace) -> Self {
        Self { face, zoom: 0, x: 0, y: 0 }
    }

    /// Returns the 4 quadtree children for this patch
    pub fn children(&self) -> [PatchKey; 4] {
        let next_zoom = self.zoom + 1;
        let nx = self.x * 2;
        let ny = self.y * 2;
        [
            PatchKey { face: self.face, zoom: next_zoom, x: nx, y: ny },
            PatchKey { face: self.face, zoom: next_zoom, x: nx + 1, y: ny },
            PatchKey { face: self.face, zoom: next_zoom, x: nx, y: ny + 1 },
            PatchKey { face: self.face, zoom: next_zoom, x: nx + 1, y: ny + 1 },
        ]
    }

    /// Computes the normalized sphere center and radius (in meters) for this patch
    pub fn compute_bounds(&self, planet_radius: f64) -> ([f64; 3], f64, [f64; 3], [f64; 3]) {
        let div = (1u64 << self.zoom) as f64;
        let u0 = (self.x as f64 / div) * 2.0 - 1.0;
        let u1 = ((self.x + 1) as f64 / div) * 2.0 - 1.0;
        let v0 = (self.y as f64 / div) * 2.0 - 1.0;
        let v1 = ((self.y + 1) as f64 / div) * 2.0 - 1.0;

        let u_mid = (u0 + u1) * 0.5;
        let v_mid = (v0 + v1) * 0.5;

        // Center on sphere
        let c_cube = self.face.to_cube_vector(u_mid, v_mid);
        let c_len = (c_cube[0] * c_cube[0] + c_cube[1] * c_cube[1] + c_cube[2] * c_cube[2]).sqrt();
        let center = [
            (c_cube[0] / c_len) * planet_radius,
            (c_cube[1] / c_len) * planet_radius,
            (c_cube[2] / c_len) * planet_radius,
        ];

        // 4 corners on sphere
        let corners = [
            self.face.to_cube_vector(u0, v0),
            self.face.to_cube_vector(u1, v0),
            self.face.to_cube_vector(u0, v1),
            self.face.to_cube_vector(u1, v1),
        ];

        let mut max_corner_dist: f64 = 0.0;
        let mut min_bound = [f64::INFINITY, f64::INFINITY, f64::INFINITY];
        let mut max_bound = [f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];

        for corner in &corners {
            let len = (corner[0] * corner[0] + corner[1] * corner[1] + corner[2] * corner[2]).sqrt();
            let p = [
                (corner[0] / len) * planet_radius,
                (corner[1] / len) * planet_radius,
                (corner[2] / len) * planet_radius,
            ];

            let dx = p[0] - center[0];
            let dy = p[1] - center[1];
            let dz = p[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > max_corner_dist {
                max_corner_dist = dist;
            }

            for i in 0..3 {
                if p[i] < min_bound[i] { min_bound[i] = p[i]; }
                if p[i] > max_bound[i] { max_bound[i] = p[i]; }
            }
        }

        (center, max_corner_dist, min_bound, max_bound)
    }
}

/// Floating-Origin Camera calculation to eliminate 32-bit floating point precision jitter.
/// Keeps the camera at the world origin (0, 0, 0) during GPU submission.
#[derive(Debug, Clone, Copy)]
pub struct FloatingOriginCamera {
    pub camera_world_pos: [f64; 3],
}

impl FloatingOriginCamera {
    pub fn new(camera_world_pos: [f64; 3]) -> Self {
        Self { camera_world_pos }
    }

    /// Calculates camera-relative position in single-precision floats (32-bit),
    /// performing double-precision (64-bit) subtraction on the CPU first.
    #[inline]
    pub fn world_to_camera_relative(&self, world_pos: [f64; 3]) -> [f32; 3] {
        [
            (world_pos[0] - self.camera_world_pos[0]) as f32,
            (world_pos[1] - self.camera_world_pos[1]) as f32,
            (world_pos[2] - self.camera_world_pos[2]) as f32,
        ]
    }
}

/// GPU storage buffer representation of a terrain patch for Vulkan / WebGPU indirect pipeline
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTerrainPatch {
    pub center_rel: [f32; 3],
    pub radius: f32,
    pub min_rel: [f32; 3],
    pub lod: u32,
    pub max_rel: [f32; 3],
    pub face_and_zoom: u32,
}

/// Active phase of the Multi-Scale Fluonder Rendering Pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiScalePhase {
    /// Planet viewed from space (alt > 80,000m / 80km).
    /// Global planetary sphere with atmospheric glow (Nishita/Bruneton).
    SpaceView,
    /// Descent through mesosphere, stratosphere, and troposphere (1,000m - 80,000m).
    /// Aggressive geocentric quadtree subdivision.
    /// Cloud layer transition (5,000m - 12,000m) with dynamic alpha blending
    /// and screen-space fog density to obscure LOD transitions.
    Descent,
    /// Ground / First-Person level (alt < 1,000m).
    /// Global planetary sphere pipeline disabled; horizon flattened.
    /// Local terrain rendering active with high-fidelity collision geometry and ground physics.
    GroundFpsView,
}

/// Dynamic atmospheric and phase state computed for the renderer
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiScaleState {
    pub phase: MultiScalePhase,
    pub altitude: f64,
    /// Alpha blend factor for the outer curved cloud shell mesh [0.0 = transparent, 1.0 = fully opaque]
    pub cloud_shell_alpha: f32,
    /// Screen-space atmospheric fog density [0.0 = clear, 1.0 = maximum fog] to mask geometry popping
    pub atmospheric_fog_density: f32,
    pub planetary_sphere_active: bool,
    pub local_terrain_active: bool,
}

pub struct QuadtreeGlobe {
    /// Maps a tile index to its corresponding GPU cluster ID
    pub loaded_tiles: HashMap<TileIndex, u32>,
    pub max_zoom: u8,
    pub planet_radius: f64,
}

impl QuadtreeGlobe {
    pub fn new(max_zoom: u8) -> Self {
        Self {
            loaded_tiles: HashMap::new(),
            max_zoom,
            planet_radius: EARTH_RADIUS_WGS84,
        }
    }

    /// Evaluates which patches should be rendered based on camera ECEF position
    /// using geocentric quadtree distance-based subdivision.
    pub fn update_geocentric_patches(&self, camera_ecef: [f64; 3], split_factor: f64) -> Vec<PatchKey> {
        let mut active_patches = Vec::new();

        for face in CubeFace::ALL {
            let root = PatchKey::root(face);
            self.subdivide_patch(root, camera_ecef, split_factor, &mut active_patches);
        }

        active_patches
    }

    fn subdivide_patch(
        &self,
        patch: PatchKey,
        camera_ecef: [f64; 3],
        split_factor: f64,
        out: &mut Vec<PatchKey>,
    ) {
        let (center, radius, _, _) = patch.compute_bounds(self.planet_radius);
        let dx = camera_ecef[0] - center[0];
        let dy = camera_ecef[1] - center[1];
        let dz = camera_ecef[2] - center[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.0);

        // Distance threshold for subdivision
        let split_distance = radius * split_factor;

        if dist < split_distance && patch.zoom < self.max_zoom {
            for child in patch.children() {
                self.subdivide_patch(child, camera_ecef, split_factor, out);
            }
        } else {
            out.push(patch);
        }
    }

    /// Transforms active patches into GPU-ready `GpuTerrainPatch` storage buffer structures
    /// using double-precision floating-origin subtraction.
    pub fn generate_gpu_patches(
        &self,
        camera: &FloatingOriginCamera,
        patches: &[PatchKey],
    ) -> Vec<GpuTerrainPatch> {
        patches
            .iter()
            .map(|patch| {
                let (center, radius, min_bound, max_bound) = patch.compute_bounds(self.planet_radius);
                let center_rel = camera.world_to_camera_relative(center);
                let min_rel = camera.world_to_camera_relative(min_bound);
                let max_rel = camera.world_to_camera_relative(max_bound);
                let face_and_zoom = ((patch.face as u32) << 8) | (patch.zoom as u32);

                GpuTerrainPatch {
                    center_rel,
                    radius: radius as f32,
                    min_rel,
                    lod: patch.zoom as u32,
                    max_rel,
                    face_and_zoom,
                }
            })
            .collect()
    }

    /// Evaluates the multi-scale rendering phase, cloud transition, and fog density.
    pub fn evaluate_multiscale_state(&self, camera_ecef: [f64; 3]) -> MultiScaleState {
        let altitude = self.calculate_altitude(camera_ecef);

        if altitude > 80_000.0 {
            // Space View: outer shell fully visible, no ground fog, planetary sphere active
            MultiScaleState {
                phase: MultiScalePhase::SpaceView,
                altitude,
                cloud_shell_alpha: 1.0,
                atmospheric_fog_density: 0.0,
                planetary_sphere_active: true,
                local_terrain_active: false,
            }
        } else if altitude >= 1_000.0 {
            // Descent: cloud layer transition between 5,000m and 12,000m
            let (cloud_alpha, fog_density) = if altitude > 12_000.0 {
                // High altitude above clouds: cloud shell visible, light haze
                let t = ((altitude - 12_000.0) / (80_000.0 - 12_000.0)).clamp(0.0, 1.0);
                (1.0, (1.0 - t as f32) * 0.15)
            } else if altitude >= 5_000.0 {
                // Inside cloud deck (5,000m - 12,000m): crossfade cloud shell and peak fog density
                let t = (altitude - 5_000.0) / (12_000.0 - 5_000.0);
                let fog = 0.85 * (1.0 - (2.0 * t - 1.0).abs() as f32); // Peak fog in mid-cloud
                (t as f32, fog.max(0.3))
            } else {
                // Below cloud deck (1,000m - 5,000m): cloud shell transparent, ground haze
                let t = ((altitude - 1_000.0) / (5_000.0 - 1_000.0)).clamp(0.0, 1.0) as f32;
                (0.0, 0.05 + t * 0.25)
            };

            MultiScaleState {
                phase: MultiScalePhase::Descent,
                altitude,
                cloud_shell_alpha: cloud_alpha,
                atmospheric_fog_density: fog_density,
                planetary_sphere_active: true,
                local_terrain_active: altitude < 5_000.0,
            }
        } else {
            // Ground / FPS View: horizon is flat, planetary sphere completely disabled
            MultiScaleState {
                phase: MultiScalePhase::GroundFpsView,
                altitude,
                cloud_shell_alpha: 0.0,
                atmospheric_fog_density: 0.05,
                planetary_sphere_active: false,
                local_terrain_active: true,
            }
        }
    }

    /// Evaluates which tiles should be loaded based on camera ECEF position (legacy WGS84 mapping)
    pub fn update_lod(&mut self, camera_ecef: [f64; 3]) -> Vec<TileIndex> {
        let mut required_tiles = Vec::new();
        required_tiles.push(TileIndex { zoom: 0, x: 0, y: 0 });

        let alt = self.calculate_altitude(camera_ecef);
        let target_zoom = if alt > 500_000.0 {
            0
        } else if alt > 100_000.0 {
            1
        } else if alt > 20_000.0 {
            2
        } else if alt > 5_000.0 {
            3
        } else {
            self.max_zoom.min(4)
        };

        if target_zoom > 0 {
            // Estimate x, y from latitude / longitude
            let (lat, lon) = self.ecef_to_lat_lon(camera_ecef);
            let n = 1u32 << target_zoom;
            let x = (((lon + 180.0) / 360.0) * n as f64).clamp(0.0, (n - 1) as f64) as u32;
            let lat_rad = lat.to_radians();
            let y = (((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n as f64)
                .clamp(0.0, (n - 1) as f64) as u32;

            required_tiles.push(TileIndex { zoom: target_zoom, x, y });
        }

        required_tiles
    }

    /// Calculates a bounded (2r + 1) x (2r + 1) tile grid centered on the camera's location.
    /// At radius r = 2, this produces the 5x5 = 25 tiles neighborhood.
    pub fn calculate_local_tile_ring(&self, camera_ecef: [f64; 3], zoom: u8, radius: i32) -> Vec<TileIndex> {
        let (lat, lon) = self.ecef_to_lat_lon(camera_ecef);
        let n = 1i64 << zoom;
        let cx = (((lon + 180.0) / 360.0) * n as f64).clamp(0.0, (n - 1) as f64) as i64;
        let lat_rad = lat.to_radians();
        let cy = (((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n as f64)
            .clamp(0.0, (n - 1) as f64) as i64;

        let mut ring = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);

        for dy in -radius..=radius {
            let ty = cy + dy as i64;
            if ty < 0 || ty >= n {
                continue; // Clamp to valid spherical Mercator bounds
            }
            for dx in -radius..=radius {
                // Wrap longitude coordinate horizontally
                let tx = (cx + dx as i64).rem_euclid(n);
                ring.push(TileIndex {
                    zoom,
                    x: tx as u32,
                    y: ty as u32,
                });
            }
        }

        ring
    }

    /// Altitude boundary transition (ECEF to Local origin shift)
    pub fn calculate_render_origin(&self, camera_ecef: [f64; 3]) -> [f64; 3] {
        let alt = self.calculate_altitude(camera_ecef);
        if alt < 1000.0 {
            // Below 1,000m, snap origin to the surface directly beneath the camera
            self.project_to_surface(camera_ecef)
        } else {
            // Above 1,000m, keep origin at Earth center
            [0.0, 0.0, 0.0]
        }
    }

    pub fn calculate_altitude(&self, ecef: [f64; 3]) -> f64 {
        let x = ecef[0];
        let y = ecef[1];
        let z = ecef[2];
        let p = (x * x + y * y).sqrt();
        let r = (p * p + z * z).sqrt();
        r - self.planet_radius
    }

    pub fn project_to_surface(&self, ecef: [f64; 3]) -> [f64; 3] {
        let x = ecef[0];
        let y = ecef[1];
        let z = ecef[2];
        let p = (x * x + y * y).sqrt();
        let scale = self.planet_radius / (p * p + z * z).sqrt();
        [x * scale, y * scale, z * scale]
    }

    pub fn ecef_to_lat_lon(&self, ecef: [f64; 3]) -> (f64, f64) {
        let x = ecef[0];
        let y = ecef[1];
        let z = ecef[2];
        let p = (x * x + y * y).sqrt();
        let lon = y.atan2(x).to_degrees();
        let lat = z.atan2(p).to_degrees();
        (lat, lon)
    }
}

/// Viewport rendering mode: full planetary globe vs localized tactical tile map
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewportMode {
    /// Globe mode: Full planetary sphere, orbital telemetry, global aircraft/satellite tracks, quadtree LOD
    Globe,
    /// Local Map / Tactical mode: Bounded (2r + 1) x (2r + 1) tile ring (e.g. 5x5 = 25 tiles),
    /// high precision ground physics, distant telemetry culled/throttled to save mobile GPU/CPU resources.
    LocalMap,
}

/// Configuration for Local Map tile ring and prefetch
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalTileRingConfig {
    /// Ring radius for rendered tiles (r = 1 => 3x3 = 9 tiles, r = 2 => 5x5 = 25 tiles)
    pub render_radius: i32,
    /// Ring radius for asynchronous prefetch cache (e.g. r = 3 => 7x7 outer lookahead)
    pub prefetch_radius: i32,
    /// Target tile zoom level for local map mode (e.g. 15 or 16 for street/tactical level)
    pub target_zoom: u8,
    /// Distance cutoff in meters for ingesting entity telemetry in LocalMap mode
    pub telemetry_cull_radius_meters: f64,
}

impl Default for LocalTileRingConfig {
    fn default() -> Self {
        Self {
            render_radius: 2,       // 5x5 = 25 tiles active
            prefetch_radius: 3,     // 7x7 outer prefetch lookahead
            target_zoom: 15,
            telemetry_cull_radius_meters: 25_000.0, // 25km local bubble
        }
    }
}

/// Manages active tiles and prefetch queue for the bounded (2r + 1) x (2r + 1) tile window.
#[derive(Debug, Clone)]
pub struct RingTileManager {
    pub config: LocalTileRingConfig,
    pub current_center: Option<TileIndex>,
    pub active_render_tiles: Vec<TileIndex>,
    pub prefetch_queue: Vec<TileIndex>,
}

impl RingTileManager {
    pub fn new(config: LocalTileRingConfig) -> Self {
        Self {
            config,
            current_center: None,
            active_render_tiles: Vec::new(),
            prefetch_queue: Vec::new(),
        }
    }

    /// Updates the active 5x5 (or (2r+1)x(2r+1)) window and outer prefetch queue based on camera coordinates
    pub fn update(&mut self, globe: &QuadtreeGlobe, camera_ecef: [f64; 3]) -> bool {
        let (lat, lon) = globe.ecef_to_lat_lon(camera_ecef);
        let zoom = self.config.target_zoom;
        let n = 1i64 << zoom;
        let cx = (((lon + 180.0) / 360.0) * n as f64).clamp(0.0, (n - 1) as f64) as i64;
        let lat_rad = lat.to_radians();
        let cy = (((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n as f64)
            .clamp(0.0, (n - 1) as f64) as i64;

        let center_tile = TileIndex {
            zoom,
            x: cx as u32,
            y: cy as u32,
        };

        let center_changed = self.current_center != Some(center_tile);
        if center_changed {
            self.current_center = Some(center_tile);

            // Compute active render ring (2 * r_render + 1)^2
            self.active_render_tiles = globe.calculate_local_tile_ring(
                camera_ecef,
                zoom,
                self.config.render_radius,
            );

            // Compute outer prefetch ring (tiles in prefetch_radius but not in render_radius)
            let full_prefetch_ring = globe.calculate_local_tile_ring(
                camera_ecef,
                zoom,
                self.config.prefetch_radius,
            );

            self.prefetch_queue = full_prefetch_ring
                .into_iter()
                .filter(|tile| !self.active_render_tiles.contains(tile))
                .collect();
        }

        center_changed
    }

    /// Evaluates if an entity at world position (ECEF) should have its telemetry processed in LocalMap mode.
    /// Distant entities (> telemetry_cull_radius_meters) are gated out to save CPU/GPU cycles.
    pub fn is_entity_in_telemetry_bubble(&self, camera_ecef: [f64; 3], entity_ecef: [f64; 3]) -> bool {
        let dx = camera_ecef[0] - entity_ecef[0];
        let dy = camera_ecef[1] - entity_ecef[1];
        let dz = camera_ecef[2] - entity_ecef[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let max_r = self.config.telemetry_cull_radius_meters;
        dist_sq <= max_r * max_r
    }
}

pub struct CelestialBody {
    pub name: String,
    pub azimuth: f64,
    pub elevation: f64,
}

pub struct AstronomyEngine;

impl AstronomyEngine {
    /// Calculate azimuth and elevation of celestial bodies relative to a GPS location
    pub fn compute_celestial_positions(lat: f64, lng: f64, timestamp: i64) -> Vec<CelestialBody> {
        let mut bodies = Vec::new();
        // Solar position algorithm (simplified placeholder)
        let days_since_j2000 = (timestamp as f64 - 946728000.0) / 86400.0;
        let l = (280.460 + 0.9856474 * days_since_j2000).to_radians();

        bodies.push(CelestialBody {
            name: "Sun".to_string(),
            azimuth: lat.to_radians() + l.cos() * std::f64::consts::PI,
            elevation: lng.to_radians() + l.sin() * std::f64::consts::PI / 4.0,
        });

        bodies.push(CelestialBody {
            name: "Moon".to_string(),
            azimuth: lat.to_radians() + (l + 1.0).cos() * std::f64::consts::PI,
            elevation: lng.to_radians() + (l + 1.0).sin() * std::f64::consts::PI / 4.0,
        });

        bodies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floating_origin_precision() {
        let planet_radius = EARTH_RADIUS_WGS84;
        let camera_world = [planet_radius + 50.0, 1000.0, 2000.0];
        let camera = FloatingOriginCamera::new(camera_world);

        let object_world = [planet_radius + 50.5, 1001.2, 1999.8];
        let rel = camera.world_to_camera_relative(object_world);

        // Delta is ~0.5m in X, 1.2m in Y, -0.2m in Z
        assert!((rel[0] - 0.5).abs() < 1e-4);
        assert!((rel[1] - 1.2).abs() < 1e-4);
        assert!((rel[2] - (-0.2)).abs() < 1e-4);
    }

    #[test]
    fn test_geocentric_quadtree_subdivision() {
        let globe = QuadtreeGlobe::new(5);
        // Camera orbiting directly along +Z axis at 10,000m altitude
        let camera_pos = [0.0, 0.0, EARTH_RADIUS_WGS84 + 10_000.0];
        let patches = globe.update_geocentric_patches(camera_pos, 2.5);

        // We should have subdivided the face closest to the camera (+Z) more deeply
        assert!(patches.len() > 6);
        let max_zoom = patches.iter().map(|p| p.zoom).max().unwrap();
        assert!(max_zoom > 0);
    }

    #[test]
    fn test_multiscale_phase_transitions() {
        let globe = QuadtreeGlobe::new(5);

        // 1. Space view (> 80km)
        let space_pos = [0.0, 0.0, EARTH_RADIUS_WGS84 + 150_000.0];
        let space_state = globe.evaluate_multiscale_state(space_pos);
        assert_eq!(space_state.phase, MultiScalePhase::SpaceView);
        assert_eq!(space_state.cloud_shell_alpha, 1.0);
        assert!(space_state.planetary_sphere_active);
        assert!(!space_state.local_terrain_active);

        // 2. Mid descent (inside cloud deck, e.g. 8,500m)
        let descent_pos = [0.0, 0.0, EARTH_RADIUS_WGS84 + 8_500.0];
        let descent_state = globe.evaluate_multiscale_state(descent_pos);
        assert_eq!(descent_state.phase, MultiScalePhase::Descent);
        assert!(descent_state.atmospheric_fog_density >= 0.3); // High fog masks popping
        assert!(descent_state.planetary_sphere_active);

        // 3. Ground view (< 1km)
        let ground_pos = [0.0, 0.0, EARTH_RADIUS_WGS84 + 50.0];
        let ground_state = globe.evaluate_multiscale_state(ground_pos);
        assert_eq!(ground_state.phase, MultiScalePhase::GroundFpsView);
        assert_eq!(ground_state.cloud_shell_alpha, 0.0);
        assert!(!ground_state.planetary_sphere_active); // Global sphere disabled
        assert!(ground_state.local_terrain_active);
    }

    #[test]
    fn test_ring_tile_manager_5x5_and_telemetry_gating() {
        let globe = QuadtreeGlobe::new(15);
        let config = LocalTileRingConfig {
            render_radius: 2,   // 5x5 = 25 tiles
            prefetch_radius: 3, // 7x7 outer lookahead = 49 - 25 = 24 prefetch tiles
            target_zoom: 15,
            telemetry_cull_radius_meters: 25_000.0,
        };

        let mut manager = RingTileManager::new(config);
        // ECEF position on equator / prime meridian
        let camera_pos = [EARTH_RADIUS_WGS84 + 100.0, 0.0, 0.0];

        let changed = manager.update(&globe, camera_pos);
        assert!(changed);
        assert_eq!(manager.active_render_tiles.len(), 25); // Exactly 5x5 = 25 tiles
        assert_eq!(manager.prefetch_queue.len(), 24);      // (7x7) - (5x5) = 24 tiles

        // Telemetry gating test
        let close_entity = [EARTH_RADIUS_WGS84 + 100.0, 5000.0, 0.0]; // ~5km away
        assert!(manager.is_entity_in_telemetry_bubble(camera_pos, close_entity));

        let distant_entity = [EARTH_RADIUS_WGS84 + 100.0, 100_000.0, 0.0]; // ~100km away
        assert!(!manager.is_entity_in_telemetry_bubble(camera_pos, distant_entity));
    }
}

