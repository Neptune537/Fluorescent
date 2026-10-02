use fluorite_core::allocator::ChunkPoolAllocator;
use fluorite_core::api::engine::simulate_massive_entity_load;
use fluorite_core::ecs::world::EngineWorld;
use fluorite_core::streaming::world_streamer::{StreamingConfig, WorldStreamingManager};
use glam::Vec3;
use std::sync::Arc;

#[test]
fn test_world_streaming_integration() {
    let mut engine_world = EngineWorld::new();
    let chunk_pool = Arc::new(ChunkPoolAllocator::new(1024 * 1024, 16).unwrap());

    let mut manager = WorldStreamingManager::new(
        StreamingConfig::default(),
        chunk_pool.clone(),
        Some(engine_world.command_sender()),
    );

    // Simulate player movement
    manager.update(Vec3::new(100.0, 0.0, 100.0), Vec3::new(10.0, 0.0, 10.0), None);

    // Verify chunks are queued for streaming
    assert!(manager.resident_chunk_count() > 0);

    // Process queued commands in ECS
    engine_world.process_streaming_commands();
}

#[test]
fn test_massive_entity_load_bridge() {
    // 100,000 entities buffer test
    let count = 100_000;
    let buffer = simulate_massive_entity_load(count);

    // Ensure size is correct: count * struct size + alignment/padding (from our allocator logic if applicable)
    // Actually our allocate_engine_buffer returns exactly count * struct size, let's just check buffer len.
    let expected_size = count * std::mem::size_of::<fluorite_core::fluoropod::gc_firewall::SanitizedUiState>();
    assert_eq!(buffer.len(), expected_size);

    // The implementation overrides the buffer with bytemuck slice filling,
    // so the sentinels might be overwritten if bytemuck fills the entire buffer.
    // The test shouldn't rely on 0xAA/0x55 being exactly at byte 0 and byte len-1 if it gets overwritten by typed_slice.fill(dummy_state).

    // We can cast it back to verify the values
    let typed_slice: &[fluorite_core::fluoropod::gc_firewall::SanitizedUiState] = bytemuck::cast_slice(&buffer);
    assert_eq!(typed_slice.len(), count);

    // Check first element
    assert_eq!(typed_slice[0].speed_kmh.round(), 36.0); // 10 m/s
    assert_eq!(typed_slice[0].altitude_meters, 15.0);
    assert_eq!(typed_slice[0].target_locked, 1);
}
