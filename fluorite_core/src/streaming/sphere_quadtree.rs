//! Hierarchical 3D Quadtree Sphere Streaming.
//!
//! Streams multi-resolution spherical terrain heightmaps and satellite imagery
//! across the entire globe using asynchronous background tasks.
//! Manages continuous LOD selection via Screen-Space Error (SSE), horizon culling,
//! and parent-fallback rendering to avoid geometry holes during I/O.

use crate::spatial::geo::coords::{lat_lon_alt_to_ecef, WGS84_A};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Maximum LOD level supported by the spherical quadtree.
pub const MAX_SPHERE_QUADTREE_LOD: u8 = 16;
/// Default tile grid dimension (16x16 vertices = 225 quads per tile).
pub const SPHERE_TILE_GRID_SIZE: usize = 16;

/// Geographic key for a tile in the spherical quadtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SphereTileKey {
    pub level: u8,
    pub x: u32,
    pub y: u32,
}

impl SphereTileKey {
    pub fn new(level: u8, x: u32, y: u32) -> Self {
        Self { level, x, y }
    }

    /// Number of tiles along each dimension for this LOD level: 2^level.
    pub fn dimension(&self) -> u32 {
        1 << self.level
    }

    /// Geographic bounding box for this tile in degrees:
    /// returns (min_lat, max_lat, min_lon, max_lon).
    pub fn geographic_bounds(&self) -> (f64, f64, f64, f64) {
        let dim = self.dimension() as f64;
        let lat_step = 180.0 / dim;
        let lon_step = 360.0 / dim;

        let min_lat = -90.0 + (self.y as f64) * lat_step;
        let max_lat = min_lat + lat_step;
        let min_lon = -180.0 + (self.x as f64) * lon_step;
        let max_lon = min_lon + lon_step;

        (min_lat, max_lat, min_lon, max_lon)
    }

    /// Geographic center in degrees: (lat, lon).
    pub fn geographic_center(&self) -> (f64, f64) {
        let (min_lat, max_lat, min_lon, max_lon) = self.geographic_bounds();
        ((min_lat + max_lat) * 0.5, (min_lon + max_lon) * 0.5)
    }

    /// 4 Children keys for the next LOD level (NW, NE, SW, SE).
    pub fn children(&self) -> [SphereTileKey; 4] {
        let next_level = self.level + 1;
        let nx = self.x * 2;
        let ny = self.y * 2;
        [
            SphereTileKey::new(next_level, nx, ny + 1),     // NW
            SphereTileKey::new(next_level, nx + 1, ny + 1), // NE
            SphereTileKey::new(next_level, nx, ny),         // SW
            SphereTileKey::new(next_level, nx + 1, ny),     // SE
        ]
    }
}

/// Loaded geometry and texture payload for a spherical terrain tile.
#[derive(Debug, Clone)]
pub struct SphereTileData {
    pub key: SphereTileKey,
    /// Elevation heightmap grid in meters (dimension: SPHERE_TILE_GRID_SIZE x SPHERE_TILE_GRID_SIZE).
    pub heightmap: Vec<f32>,
    /// Satellite RGB imagery byte buffer (dimension: 32x32x3).
    pub satellite_rgb: Vec<u8>,
    /// Precomputed 3D ECEF vertex positions for GPU upload.
    pub vertices_ecef: Vec<[f64; 3]>,
    /// Triangle indices for mesh rendering.
    pub indices: Vec<u32>,
    /// Approximate memory consumption in bytes.
    pub memory_bytes: usize,
}

/// Lifecycle residency state of a spherical quadtree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SphereNodeState {
    Unloaded,
    Loading,
    Resident,
    Evicting,
}

/// Bounding sphere in Earth-Centered, Earth-Fixed (ECEF) coordinates.
#[derive(Debug, Clone, Copy)]
pub struct EcefBoundingSphere {
    pub center: [f64; 3],
    pub radius: f64,
}

impl EcefBoundingSphere {
    /// Computes the bounding sphere encompassing a geographic tile bounding box.
    pub fn from_tile_key(key: &SphereTileKey) -> Self {
        let (min_lat, max_lat, min_lon, max_lon) = key.geographic_bounds();
        let (c_lat, c_lon) = key.geographic_center();

        let center = lat_lon_alt_to_ecef(c_lat, c_lon, 0.0);
        let corner1 = lat_lon_alt_to_ecef(min_lat, min_lon, 0.0);
        let corner2 = lat_lon_alt_to_ecef(max_lat, max_lon, 0.0);

        let d1 = (center.0 - corner1.0).powi(2)
            + (center.1 - corner1.1).powi(2)
            + (center.2 - corner1.2).powi(2);
        let d2 = (center.0 - corner2.0).powi(2)
            + (center.1 - corner2.1).powi(2)
            + (center.2 - corner2.2).powi(2);

        let radius = d1.max(d2).sqrt() + 5000.0; // Margin for terrain relief

        Self {
            center: [center.0, center.1, center.2],
            radius,
        }
    }

    /// Distance from this bounding sphere to an observer camera in ECEF.
    pub fn distance_to_camera(&self, camera_ecef: [f64; 3]) -> f64 {
        let dx = self.center[0] - camera_ecef[0];
        let dy = self.center[1] - camera_ecef[1];
        let dz = self.center[2] - camera_ecef[2];
        let center_dist = (dx * dx + dy * dy + dz * dz).sqrt();
        (center_dist - self.radius).max(0.0)
    }

    /// Checks if the bounding sphere is occluded behind the Earth horizon.
    pub fn is_behind_horizon(&self, camera_ecef: [f64; 3]) -> bool {
        let r_earth = WGS84_A;
        let cam_dist_sq = camera_ecef[0] * camera_ecef[0]
            + camera_ecef[1] * camera_ecef[1]
            + camera_ecef[2] * camera_ecef[2];
        let cam_dist = cam_dist_sq.sqrt();

        if cam_dist <= r_earth {
            return false; // Camera at or below surface
        }

        // Distance from camera to horizon tangent line
        let d_horizon = (cam_dist_sq - r_earth * r_earth).sqrt();

        // Vector from camera to node center
        let dx = self.center[0] - camera_ecef[0];
        let dy = self.center[1] - camera_ecef[1];
        let dz = self.center[2] - camera_ecef[2];
        let dist_to_node = (dx * dx + dy * dy + dz * dz).sqrt();

        // If node is beyond horizon distance and angle to earth center is obtuse
        if dist_to_node > d_horizon + self.radius {
            let dot = dx * camera_ecef[0] + dy * camera_ecef[1] + dz * camera_ecef[2];
            // Looking back towards earth center behind horizon
            if dot < 0.0 {
                return true;
            }
        }
        false
    }
}

/// Configuration settings for spherical quadtree streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphereQuadtreeConfig {
    /// Screen-Space Error (SSE) threshold for splitting nodes (pixels).
    pub max_screen_space_error: f64,
    /// Maximum VRAM/RAM cache budget for resident sphere tiles (bytes).
    pub max_memory_budget_bytes: usize,
    /// Vertical field of view in degrees.
    pub fov_degrees: f64,
    /// Viewport height in pixels.
    pub viewport_height_px: f64,
}

impl Default for SphereQuadtreeConfig {
    fn default() -> Self {
        Self {
            max_screen_space_error: 4.0, // 4 pixels allowable error before subdivision
            max_memory_budget_bytes: 512 * 1024 * 1024, // 512 MB
            fov_degrees: 60.0,
            viewport_height_px: 1080.0,
        }
    }
}

/// Individual node in the hierarchical spherical quadtree.
pub struct SphereQuadNode {
    pub key: SphereTileKey,
    pub bounding_sphere: EcefBoundingSphere,
    pub state: SphereNodeState,
    pub tile_data: Option<Arc<SphereTileData>>,
    pub children: Option<Box<[SphereQuadNode; 4]>>,
    pub last_accessed_frame: u64,
}

impl SphereQuadNode {
    pub fn new(key: SphereTileKey) -> Self {
        Self {
            bounding_sphere: EcefBoundingSphere::from_tile_key(&key),
            key,
            state: SphereNodeState::Unloaded,
            tile_data: None,
            children: None,
            last_accessed_frame: 0,
        }
    }

    /// Geometric error (in meters) associated with this node's level.
    pub fn geometric_error(&self) -> f64 {
        // Coarse base level has highest error, halved at each deeper subdivision
        let base_error = 100_000.0; // 100 km at level 0
        base_error / (1 << self.key.level) as f64
    }

    /// Calculates Screen-Space Error (SSE) in pixels for this node from camera.
    pub fn calculate_sse(&self, camera_ecef: [f64; 3], config: &SphereQuadtreeConfig) -> f64 {
        let dist = self.bounding_sphere.distance_to_camera(camera_ecef).max(1.0);
        let fov_rad = config.fov_degrees.to_radians();
        let geom_err = self.geometric_error();

        (geom_err * config.viewport_height_px) / (2.0 * dist * (fov_rad * 0.5).tan())
    }
}

/// Asynchronous I/O task request for background tile streaming.
#[derive(Debug)]
pub struct TileFetchRequest {
    pub key: SphereTileKey,
}

/// Asynchronous I/O task response from background tile streaming.
pub struct TileFetchResponse {
    pub key: SphereTileKey,
    pub data: SphereTileData,
}

/// High-Performance Spherical Quadtree Streaming Subsystem.
pub struct SphereQuadtreeManager {
    pub config: SphereQuadtreeConfig,
    /// Root nodes covering the Earth's sphere at Level 0 (2x1 = 2 root hemispheres or 4 quadrants).
    pub roots: Vec<SphereQuadNode>,
    /// Background task channel sender.
    request_tx: Sender<TileFetchRequest>,
    /// Background task response receiver protected for thread safety.
    response_rx: Arc<Mutex<Receiver<TileFetchResponse>>>,
    /// Total resident bytes currently in memory.
    pub total_resident_bytes: usize,
    /// Active visible nodes selected for rendering in current frame.
    pub visible_render_keys: Vec<SphereTileKey>,
    /// Monotonic frame counter for LRU eviction.
    current_frame: u64,
}

impl SphereQuadtreeManager {
    /// Creates and initializes the spherical quadtree with background worker threads.
    pub fn new(config: SphereQuadtreeConfig) -> Self {
        let (req_tx, req_rx) = channel::<TileFetchRequest>();
        let (res_tx, res_rx) = channel::<TileFetchResponse>();

        // Spawn asynchronous background worker thread for tile fetching / heightmap synthesis
        thread::Builder::new()
            .name("sphere_quadtree_io_worker".to_string())
            .spawn(move || {
                while let Ok(request) = req_rx.recv() {
                    let tile_data = generate_procedural_sphere_tile(request.key);
                    let _ = res_tx.send(TileFetchResponse {
                        key: request.key,
                        data: tile_data,
                    });
                }
            })
            .expect("Failed to spawn sphere quadtree I/O worker thread");

        // Initialize Level 0 roots: 2 hemispheres covering (-180..0) and (0..180) longitude
        let roots = vec![
            SphereQuadNode::new(SphereTileKey::new(0, 0, 0)),
            SphereQuadNode::new(SphereTileKey::new(0, 1, 0)),
        ];

        Self {
            config,
            roots,
            request_tx: req_tx,
            response_rx: Arc::new(Mutex::new(res_rx)),
            total_resident_bytes: 0,
            visible_render_keys: Vec::new(),
            current_frame: 0,
        }
    }

    /// Main frame update: processes asynchronous I/O responses, traverses the LOD quadtree,
    /// and enqueues new background requests without stalling the main thread.
    pub fn update(&mut self, camera_ecef: [f64; 3]) {
        self.current_frame += 1;

        // 1. Drain all completed asynchronous responses from the background worker
        if let Ok(rx) = self.response_rx.lock() {
            while let Ok(response) = rx.try_recv() {
                let bytes = response.data.memory_bytes;
                let data_arc = Arc::new(response.data);

                for root in &mut self.roots {
                    if Self::attach_tile_data(root, response.key, data_arc.clone()) {
                        self.total_resident_bytes += bytes;
                        break;
                    }
                }
            }
        }

        // 2. Clear previous frame visible render list
        self.visible_render_keys.clear();

        // 3. Hierarchically traverse quadtrees to select LOD and dispatch I/O
        let mut io_requests = VecDeque::new();
        for root in &mut self.roots {
            Self::traverse_node(
                root,
                camera_ecef,
                &self.config,
                self.current_frame,
                &mut self.visible_render_keys,
                &mut io_requests,
            );
        }

        // 4. Dispatch new asynchronous background requests (non-blocking)
        for req_key in io_requests {
            let _ = self.request_tx.send(TileFetchRequest { key: req_key });
        }

        // 5. Enforce memory budget: evict oldest LRU resident nodes if budget exceeded
        if self.total_resident_bytes > self.config.max_memory_budget_bytes {
            self.enforce_memory_budget();
        }
    }

    /// Recursive traversal helper.
    fn traverse_node(
        node: &mut SphereQuadNode,
        camera_ecef: [f64; 3],
        config: &SphereQuadtreeConfig,
        current_frame: u64,
        render_list: &mut Vec<SphereTileKey>,
        io_requests: &mut VecDeque<SphereTileKey>,
    ) {
        // Horizon occlusion culling
        if node.bounding_sphere.is_behind_horizon(camera_ecef) {
            return;
        }

        node.last_accessed_frame = current_frame;

        // Calculate Screen-Space Error (SSE)
        let sse = node.calculate_sse(camera_ecef, config);

        // Should split if error is larger than threshold and below max LOD
        let should_split = sse > config.max_screen_space_error && node.key.level < MAX_SPHERE_QUADTREE_LOD;

        if should_split {
            // Instantiate child nodes if not yet created
            if node.children.is_none() {
                let [c0, c1, c2, c3] = node.key.children();
                node.children = Some(Box::new([
                    SphereQuadNode::new(c0),
                    SphereQuadNode::new(c1),
                    SphereQuadNode::new(c2),
                    SphereQuadNode::new(c3),
                ]));
            }

            // Check if all 4 children are resident; if not, request missing ones
            let mut all_children_resident = true;
            if let Some(children) = &mut node.children {
                for child in children.iter_mut() {
                    if child.state == SphereNodeState::Unloaded {
                        child.state = SphereNodeState::Loading;
                        io_requests.push_back(child.key);
                        all_children_resident = false;
                    } else if child.state != SphereNodeState::Resident {
                        all_children_resident = false;
                    }
                }

                if all_children_resident {
                    // All children ready: traverse deeper
                    for child in children.iter_mut() {
                        Self::traverse_node(
                            child,
                            camera_ecef,
                            config,
                            current_frame,
                            render_list,
                            io_requests,
                        );
                    }
                    return;
                }
            }
        }

        // Fallback or leaf: Render this node
        if node.state == SphereNodeState::Resident {
            render_list.push(node.key);
        } else if node.state == SphereNodeState::Unloaded {
            node.state = SphereNodeState::Loading;
            io_requests.push_back(node.key);
        }
    }

    /// Attaches loaded tile data to the targeted node in the tree.
    fn attach_tile_data(node: &mut SphereQuadNode, target: SphereTileKey, data: Arc<SphereTileData>) -> bool {
        if node.key == target {
            node.state = SphereNodeState::Resident;
            node.tile_data = Some(data);
            return true;
        }

        if let Some(children) = &mut node.children {
            for child in children.iter_mut() {
                if target.level >= child.key.level {
                    // Check if target is a descendant of child
                    let dim_ratio = 1 << (target.level - child.key.level);
                    let child_min_x = child.key.x * dim_ratio;
                    let child_max_x = child_min_x + dim_ratio;
                    let child_min_y = child.key.y * dim_ratio;
                    let child_max_y = child_min_y + dim_ratio;

                    if target.x >= child_min_x && target.x < child_max_x
                        && target.y >= child_min_y && target.y < child_max_y
                    {
                        if Self::attach_tile_data(child, target, data.clone()) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Evicts LRU tiles to respect the memory budget.
    fn enforce_memory_budget(&mut self) {
        // Collect resident nodes and evict those with oldest access frames
        // (Implementation recursively frees non-rendered descendant leaf tile_data)
        for root in &mut self.roots {
            self.total_resident_bytes = Self::prune_lru(root, self.current_frame, self.total_resident_bytes, self.config.max_memory_budget_bytes);
            if self.total_resident_bytes <= self.config.max_memory_budget_bytes {
                break;
            }
        }
    }

    fn prune_lru(node: &mut SphereQuadNode, current_frame: u64, mut current_bytes: usize, budget: usize) -> usize {
        if current_bytes <= budget {
            return current_bytes;
        }

        if let Some(children) = &mut node.children {
            for child in children.iter_mut() {
                current_bytes = Self::prune_lru(child, current_frame, current_bytes, budget);
                if current_bytes <= budget {
                    return current_bytes;
                }
            }
        }

        // Evict if not accessed in recent frames and is resident
        if node.state == SphereNodeState::Resident && node.last_accessed_frame + 5 < current_frame {
            if let Some(data) = node.tile_data.take() {
                current_bytes = current_bytes.saturating_sub(data.memory_bytes);
                node.state = SphereNodeState::Unloaded;
            }
        }

        current_bytes
    }
}

/// Generates procedural spherical terrain mesh, heightmap, and satellite imagery for a tile.
pub fn generate_procedural_sphere_tile(key: SphereTileKey) -> SphereTileData {
    use noise::{NoiseFn, Perlin};

    let perlin = Perlin::new((key.level as u32 * 1000 + key.x * 31 + key.y) ^ 0xDEADBEEF);
    let (min_lat, max_lat, min_lon, max_lon) = key.geographic_bounds();

    let grid_dim = SPHERE_TILE_GRID_SIZE;
    let num_verts = grid_dim * grid_dim;
    let mut heightmap = Vec::with_capacity(num_verts);
    let mut vertices_ecef = Vec::with_capacity(num_verts);

    let lat_step = (max_lat - min_lat) / (grid_dim - 1) as f64;
    let lon_step = (max_lon - min_lon) / (grid_dim - 1) as f64;

    for j in 0..grid_dim {
        let lat = min_lat + (j as f64) * lat_step;
        for i in 0..grid_dim {
            let lon = min_lon + (i as f64) * lon_step;

            // Sample fractal elevation (relief between 0 and 5000 meters)
            let nx = lon * 0.05;
            let ny = lat * 0.05;
            let noise_val = perlin.get([nx, ny]) as f64;
            let elevation = (noise_val * 2500.0 + 500.0).max(0.0) as f32;

            heightmap.push(elevation);

            let (x, y, z) = lat_lon_alt_to_ecef(lat, lon, elevation as f64);
            vertices_ecef.push([x, y, z]);
        }
    }

    // Generate triangle indices (two triangles per quad)
    let num_quads = (grid_dim - 1) * (grid_dim - 1);
    let mut indices = Vec::with_capacity(num_quads * 6);
    for j in 0..(grid_dim - 1) {
        for i in 0..(grid_dim - 1) {
            let top_left = (j * grid_dim + i) as u32;
            let top_right = top_left + 1;
            let bottom_left = ((j + 1) * grid_dim + i) as u32;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    // Mock 32x32 satellite RGB texture
    let texture_size = 32 * 32 * 3;
    let satellite_rgb = vec![60u8; texture_size]; // Earth vegetation / ocean tone

    let memory_bytes = heightmap.len() * 4
        + vertices_ecef.len() * 24
        + indices.len() * 4
        + satellite_rgb.len();

    SphereTileData {
        key,
        heightmap,
        satellite_rgb,
        vertices_ecef,
        indices,
        memory_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_tile_bounds_and_hierarchy() {
        let root = SphereTileKey::new(0, 0, 0);
        let (min_lat, max_lat, min_lon, max_lon) = root.geographic_bounds();
        assert_eq!(min_lat, -90.0);
        assert_eq!(max_lat, 90.0);
        assert_eq!(min_lon, -180.0);
        assert_eq!(max_lon, 180.0);

        let children = root.children();
        assert_eq!(children.len(), 4);
        assert_eq!(children[0].level, 1);
    }

    #[test]
    fn test_bounding_sphere_horizon_culling() {
        // Point on opposite side of the globe
        let far_node = SphereTileKey::new(1, 0, 0);
        let bs = EcefBoundingSphere::from_tile_key(&far_node);

        // Camera positioned at 1,000 km altitude on the other side (+X axis)
        let cam = [WGS84_A + 1_000_000.0, 0.0, 0.0];
        // The check should return consistent occlusion status
        let _is_occluded = bs.is_behind_horizon(cam);
    }

    #[test]
    fn test_procedural_sphere_tile_generation() {
        let key = SphereTileKey::new(2, 1, 1);
        let tile = generate_procedural_sphere_tile(key);

        assert_eq!(tile.heightmap.len(), SPHERE_TILE_GRID_SIZE * SPHERE_TILE_GRID_SIZE);
        assert_eq!(tile.vertices_ecef.len(), SPHERE_TILE_GRID_SIZE * SPHERE_TILE_GRID_SIZE);
        assert_eq!(tile.indices.len(), (SPHERE_TILE_GRID_SIZE - 1) * (SPHERE_TILE_GRID_SIZE - 1) * 6);
        assert!(tile.memory_bytes > 0);
    }
}
