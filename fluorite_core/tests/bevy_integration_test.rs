use fluorite_core::ecs::bevy_adapter::BevyEngineWorld;
use fluorite_core::ecs::components::geo::GeographicPosition;
use fluorite_core::ecs::reflection::EngineTypeRegistry;
use fluorite_core::ecs::world::TransformComponent;

#[test]
fn test_bevy_ecs_batch_spawning_and_archetypes() {
    let mut world = BevyEngineWorld::new();

    let transforms = vec![
        TransformComponent { matrix: [1.0; 16] },
        TransformComponent { matrix: [2.0; 16] },
        TransformComponent { matrix: [3.0; 16] },
    ];

    let entities = world.spawn_batch_transforms(transforms);
    assert_eq!(entities.len(), 3);
    assert_eq!(world.entity_count(), 3);
}

#[test]
fn test_bevy_ecs_change_detection_harvesting() {
    let mut world = BevyEngineWorld::new();

    // 1. Spawn an initial spatial entity
    let entity = world.spawn_spatial_entity(
        TransformComponent { matrix: [0.0; 16] },
        Some(GeographicPosition::new(37.7749, -122.4194, 10.0)),
    );

    // Initial check: freshly spawned component is marked Changed in Bevy ECS
    let changed_packets = world.harvest_changed_transforms();
    assert_eq!(changed_packets.len(), 1);
    assert_eq!(changed_packets[0].entity_id, entity.to_bits());

    // Advance/step the world tick
    world.step();

    // Now, without any mutations, change detection should report 0 changed transforms
    let no_changes = world.harvest_changed_transforms();
    assert_eq!(no_changes.len(), 0);

    // Mutate the component directly
    if let Some(mut transform) = world.world.get_mut::<TransformComponent>(entity) {
        transform.matrix[0] = 42.0;
    }

    // Now harvesting should catch ONLY the mutated entity
    let mutated_packets = world.harvest_changed_transforms();
    assert_eq!(mutated_packets.len(), 1);
    assert_eq!(mutated_packets[0].entity_id, entity.to_bits());
    assert_eq!(mutated_packets[0].matrix[0], 42.0);
}

#[test]
fn test_bevy_reflect_serialization_and_inspection() {
    let registry = EngineTypeRegistry::new();

    let pos = GeographicPosition::new(51.5074, -0.1278, 15.5);

    // Test JSON serialization
    let json_str = registry.serialize_to_json(&pos).expect("Serialization failed");
    assert!(json_str.contains("51.5074"));
    assert!(json_str.contains("-0.1278"));

    // Test JSON deserialization into dynamic reflection
    let deserialized = registry.deserialize_from_json(&json_str).expect("Deserialization failed");
    assert!(deserialized.reflect_type_path() == "fluorite_core::ecs::components::geo::GeographicPosition" || deserialized.reflect_type_path() == "bevy_reflect::DynamicStruct");

    // Test dynamic field inspection
    let lat_val = EngineTypeRegistry::inspect_field(&pos, "lat");
    assert_eq!(lat_val, Some("51.5074".to_string()));

    let alt_val = EngineTypeRegistry::inspect_field(&pos, "alt");
    assert_eq!(alt_val, Some("15.5".to_string()));
}

#[test]
fn test_bevy_reflect_dynamic_mutation() {
    let mut transform = TransformComponent { matrix: [0.0; 16] };

    // Inspect matrix field
    let field = EngineTypeRegistry::inspect_field(&transform, "matrix");
    assert!(field.is_some());

    // Dynamic mutation using another value
    let new_matrix: [f32; 16] = [99.0; 16];
    let res = EngineTypeRegistry::mutate_field(&mut transform, "matrix", &new_matrix);
    assert!(res.is_ok());
    assert_eq!(transform.matrix[0], 99.0);
    assert_eq!(transform.matrix[15], 99.0);
}

#[test]
fn test_bevy_math_isometry_and_transforms() {
    #[allow(unused_imports)]
    use bevy_math::*;

    // Test Isometry3d translation and rotation composition
    let translation = glam::Vec3::new(10.0, 20.0, 30.0);
    let rotation = glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let iso = glam::Affine3A::from_rotation_translation(rotation, translation);

    let local_point = glam::Vec3::new(1.0, 0.0, 0.0);
    let world_point = iso.transform_point3(local_point);

    // After 90 deg rotation around Y, (1, 0, 0) points towards (0, 0, -1)
    assert!((world_point.x - 10.0).abs() < 1e-4);
    assert!((world_point.y - 20.0).abs() < 1e-4);
    assert!((world_point.z - 29.0).abs() < 1e-4);

    // Test Dir3 normalized direction representation
    let dir = Dir3::new(Vec3::new(0.0, 1.0, 0.0)).expect("Normalized vector");
    assert_eq!(*dir, Vec3::Y);
}
