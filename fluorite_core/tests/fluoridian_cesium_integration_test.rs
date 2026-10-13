use fluorite_core::ecs::components::geo::{Appearance, Controller, GeographicPosition, Visibility, WorldEntity};
use fluorite_core::spatial::geo::coords::{
    lat_lon_alt_to_cesium_cartesian3, lat_lon_alt_to_cesium_cartographic,
};
use fluorite_core::{
    CesiumImageryProvider, CesiumMapConfig, CesiumRoutePlan, CesiumRouteWaypoint,
    CesiumTerrainProvider,
};

#[test]
fn test_cesium_cartesian3_and_cartographic_parity() {
    let geo = GeographicPosition::new(37.7749, -122.4194, 50.0);

    // Test conversion via coords helper
    let cartesian = lat_lon_alt_to_cesium_cartesian3(geo.lat, geo.lon, geo.alt);
    let cartographic = lat_lon_alt_to_cesium_cartographic(geo.lat, geo.lon, geo.alt);

    assert_eq!(cartographic.latitude, 37.7749);
    assert_eq!(cartographic.longitude, -122.4194);
    assert_eq!(cartographic.height, 50.0);

    // Test conversion via GeographicPosition method
    let geo_cartesian = geo.to_cesium_cartesian3();
    assert_eq!(cartesian.x, geo_cartesian.x);
    assert_eq!(cartesian.y, geo_cartesian.y);
    assert_eq!(cartesian.z, geo_cartesian.z);

    // Verify distance from Earth's center is approximately 6378km + 50m
    let dist = (cartesian.x * cartesian.x + cartesian.y * cartesian.y + cartesian.z * cartesian.z).sqrt();
    assert!((dist - 6378187.0).abs() < 25000.0); // Within ellipsoid range
}

#[test]
fn test_world_entity_czml_packet_generation() {
    let entity = WorldEntity::new(
        1001,
        34.0522,
        -118.2437,
        15.0,
        Visibility::Exact,
        Controller::Human,
        Appearance::Model("player.gltf".to_string()),
    );

    let czml = entity.to_czml_packet();
    assert_eq!(czml.id, "node:1001");
    let json = czml.to_json().expect("CZML serialization should succeed");

    assert!(json.contains("node:1001"));
    assert!(json.contains("-118.2437"));
    assert!(json.contains("34.0522"));
    assert!(json.contains("cartographicDegrees"));
}

#[test]
fn test_cesium_map_config_tactical_and_routing() {
    let config = CesiumMapConfig::tactical_dark_preset();
    assert_eq!(config.terrain, CesiumTerrainProvider::CesiumWorldTerrain);
    assert_eq!(config.imagery, CesiumImageryProvider::CartoDbDarkMatter);
    assert!(config.enable_lighting);
    assert!(config.enable_atmosphere);

    let route = CesiumRoutePlan {
        provider: "OSRM".to_string(),
        total_distance_m: 2500.0,
        total_duration_s: 300.0,
        waypoints: vec![
            CesiumRouteWaypoint {
                longitude: -122.4194,
                latitude: 37.7749,
                elevation_m: 10.0,
            },
            CesiumRouteWaypoint {
                longitude: -122.4200,
                latitude: 37.7755,
                elevation_m: 12.0,
            },
        ],
    };

    let keyframes = route.generate_fps_camera_keyframes(60.0);
    assert_eq!(keyframes.len(), 2);
    assert_eq!(keyframes[0].speed_kmh, 60.0);
    assert!((keyframes[0].altitude_m - 11.8).abs() < 1e-3);
}
