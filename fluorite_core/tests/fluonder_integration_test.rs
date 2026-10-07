//! Comprehensive Integration Test Suite for Fluonder Engine Systems:
//! 1. Hierarchical 3D Quadtree Sphere Streaming
//! 2. Altitude Boundary Transition (~500m) & Rapier3D Terrain Colliders
//! 3. Ground-Level Celestial Placement & Astronomy Engine

use fluorite_core::physics::PhysicsWorld;
use fluorite_core::spatial::geo::altitude_transition::{
    AltitudeTransitionManager, ViewportScaleMode, ALTITUDE_TRANSITION_THRESHOLD_METERS,
};
use fluorite_core::spatial::geo::celestial::{
    compute_celestial_placements, compute_sky_illumination,
    CelestialType,
};
use fluorite_core::spatial::geo::coords::lat_lon_alt_to_ecef;
use fluorite_core::streaming::sphere_quadtree::{
    SphereQuadtreeConfig, SphereQuadtreeManager, MAX_SPHERE_QUADTREE_LOD,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_hierarchical_sphere_quadtree_streaming_lifecycle() {
    let mut config = SphereQuadtreeConfig::default();
    config.max_screen_space_error = 2.0; // Tighter error threshold to encourage subdivision
    config.max_memory_budget_bytes = 64 * 1024 * 1024; // 64 MB budget

    let mut manager = SphereQuadtreeManager::new(config);

    // Initial state: Level 0 roots exist
    assert_eq!(manager.roots.len(), 2);
    assert_eq!(manager.roots[0].key.level, 0);

    // Camera placed in High Earth Orbit: 10,000 km altitude above the equator (0, 0)
    let (cam_x, cam_y, cam_z) = lat_lon_alt_to_ecef(0.0, 0.0, 10_000_000.0);
    manager.update([cam_x, cam_y, cam_z]);

    // Give asynchronous background worker thread time to process initial tile requests
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(20));
        manager.update([cam_x, cam_y, cam_z]);
        if manager.total_resident_bytes > 0 {
            break;
        }
    }

    // Check that visible render keys are populated
    assert!(!manager.visible_render_keys.is_empty(), "Visible render keys should not be empty");
    assert!(manager.total_resident_bytes > 0, "Resident bytes should be tracked");

    // Camera zooms down closer (100 km altitude over 0.0, 0.0)
    let (close_x, close_y, close_z) = lat_lon_alt_to_ecef(0.0, 0.0, 100_000.0);

    let mut has_deeper_lod = false;
    for _ in 0..20 {
        manager.update([close_x, close_y, close_z]);
        thread::sleep(Duration::from_millis(25));
        manager.update([close_x, close_y, close_z]);
        if manager.visible_render_keys.iter().any(|k| k.level > 0) {
            has_deeper_lod = true;
            break;
        }
    }

    assert!(has_deeper_lod, "Camera proximity should trigger deeper LOD nodes");

    // Ensure LOD level is bounded by MAX_SPHERE_QUADTREE_LOD
    for key in &manager.visible_render_keys {
        assert!(key.level <= MAX_SPHERE_QUADTREE_LOD);
    }
}

#[test]
fn test_altitude_boundary_transition_and_physics_collider() {
    let mut manager = AltitudeTransitionManager::new(ALTITUDE_TRANSITION_THRESHOLD_METERS);
    let mut physics = PhysicsWorld::new();

    // Verify initial conditions: Orbital mode, 0 colliders in physics world
    assert_eq!(manager.mode, ViewportScaleMode::Orbital);
    assert_eq!(physics.num_colliders(), 0);

    // 1. High altitude: 50,000 meters (Orbital)
    let s1 = manager.update(34.0522, -118.2437, 50_000.0, &mut physics, None);
    assert_eq!(s1.mode, ViewportScaleMode::Orbital);
    assert_eq!(s1.blend_factor, 0.0);
    assert!(!s1.is_physics_collider_active);
    assert_eq!(physics.num_colliders(), 0);

    // 2. Approaching boundary: 800 meters (Transitioning zone)
    let s2 = manager.update(34.0522, -118.2437, 800.0, &mut physics, None);
    assert_eq!(s2.mode, ViewportScaleMode::Transitioning);
    assert!(s2.blend_factor > 0.0 && s2.blend_factor < 1.0);
    assert!(!s2.is_physics_collider_active);

    // 3. Crossing below boundary: 250 meters (GroundField)
    // Supply a synthetic 8x8 heightmap
    let heightmap = vec![5.0f32; 64];
    let s3 = manager.update(34.0522, -118.2437, 250.0, &mut physics, Some((&heightmap, 8, 8, 25.0)));
    assert_eq!(s3.mode, ViewportScaleMode::GroundField);
    assert_eq!(s3.blend_factor, 1.0);
    assert!(s3.is_physics_collider_active);
    assert_eq!(physics.num_colliders(), 1, "Physics world must have active terrain collider");

    // Origin must be locked to observer location
    assert!((s3.local_origin_geo.0 - 34.0522).abs() < 1e-4);
    assert!((s3.local_origin_geo.1 - -118.2437).abs() < 1e-4);

    // 4. Low hover within hysteresis band: 515 meters (stays in GroundField)
    let s4 = manager.update(34.0522, -118.2437, 515.0, &mut physics, None);
    assert_eq!(s4.mode, ViewportScaleMode::GroundField);
    assert_eq!(physics.num_colliders(), 1);

    // 5. Climbing back to orbit: 2,500 meters (exits to Orbital)
    let s5 = manager.update(34.0522, -118.2437, 2_500.0, &mut physics, None);
    assert_eq!(s5.mode, ViewportScaleMode::Orbital);
    assert!(!s5.is_physics_collider_active);
    assert_eq!(physics.num_colliders(), 0, "Terrain collider must be cleanly dismantled");
}

#[test]
fn test_celestial_placement_and_sky_illumination() {
    let lat = 37.7749; // San Francisco
    let lon = -122.4194;
    let alt = 15.0;

    // Use a fixed timestamp: 2026-06-21 20:00:00 UTC (Summer Solstice, afternoon in SF)
    let time_ms: i64 = 1782072000000;

    let placements = compute_celestial_placements(lat, lon, alt, time_ms);
    assert!(!placements.is_empty());

    // Verify Sun placement
    let sun = placements.iter().find(|p| p.body_type == CelestialType::Sun).expect("Sun not found");
    assert!(sun.azimuth_deg >= 0.0 && sun.azimuth_deg < 360.0);
    assert!(sun.altitude_deg >= -90.0 && sun.altitude_deg <= 90.0);

    // Verify direction vectors are normalized
    let cam_dir = sun.direction_camera;
    let len_sq = cam_dir[0] * cam_dir[0] + cam_dir[1] * cam_dir[1] + cam_dir[2] * cam_dir[2];
    assert!((len_sq - 1.0).abs() < 1e-3, "Camera direction vector must be unit length");

    // Verify Moon placement
    let moon = placements.iter().find(|p| p.body_type == CelestialType::Moon).expect("Moon not found");
    assert!(moon.azimuth_deg >= 0.0 && moon.azimuth_deg < 360.0);

    // Verify stars
    let polaris = placements.iter().find(|p| p.name == "Polaris").expect("Polaris not found");
    assert!(polaris.is_visible, "Polaris should be visible from mid-northern latitudes");
    assert!((polaris.altitude_deg - lat).abs() < 2.0, "Polaris altitude should match observer latitude");

    // Sky illumination
    let illumination = compute_sky_illumination(lat, lon, time_ms);
    assert!(["Day", "Civil Twilight", "Nautical Twilight", "Astronomical Twilight", "Night"].contains(&illumination.twilight_phase.as_str()));
    assert!(illumination.daylight_factor >= 0.0 && illumination.daylight_factor <= 1.0);
}
