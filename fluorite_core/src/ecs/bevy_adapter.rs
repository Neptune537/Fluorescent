//! Bevy ECS Adapter for Fluorescent.
//!
//! Provides archetype-based entity storage, parallel system scheduling,
//! and change detection to optimize data transfer across the Flutter/Rust FFI boundary.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedule;
use bevy_ecs::world::World as BevyWorld;

use crate::ecs::components::geo::GeographicPosition;
use crate::ecs::world::TransformComponent;

/// Represents a change-detected update for an entity's transform.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformSyncPacket {
    pub entity_id: u64,
    pub matrix: [f32; 16],
}

/// An archetype-based ECS world powered by Bevy ECS, complementing Fluorescent's core systems.
pub struct BevyEngineWorld {
    pub world: BevyWorld,
    pub update_schedule: Schedule,
}

impl Default for BevyEngineWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl BevyEngineWorld {
    /// Creates a new `BevyEngineWorld` with a clean world and default schedule.
    pub fn new() -> Self {
        let world = BevyWorld::new();
        let update_schedule = Schedule::default();
        Self {
            world,
            update_schedule,
        }
    }

    /// Spawns an entity with a TransformComponent and optional GeographicPosition.
    pub fn spawn_spatial_entity(
        &mut self,
        transform: TransformComponent,
        geo: Option<GeographicPosition>,
    ) -> Entity {
        let mut entity_builder = self.world.spawn(transform);
        if let Some(pos) = geo {
            entity_builder.insert(pos);
        }
        entity_builder.id()
    }

    /// Runs the configured update schedule and advances the change tracking generation.
    pub fn step(&mut self) {
        self.update_schedule.run(&mut self.world);
        self.world.clear_trackers();
    }

    /// Harvests only transforms that were modified during the latest frame.
    /// This change detection ensures zero redundant copies across the FFI bridge to Flutter.
    pub fn harvest_changed_transforms(&mut self) -> Vec<TransformSyncPacket> {
        let mut query = self
            .world
            .query_filtered::<(Entity, &TransformComponent), Changed<TransformComponent>>();

        let mut packets = Vec::new();
        for (entity, transform) in query.iter(&self.world) {
            packets.push(TransformSyncPacket {
                entity_id: entity.to_bits(),
                matrix: transform.matrix,
            });
        }
        packets
    }

    /// Batch spawns a large number of entities in a single archetype to maximize cache locality.
    pub fn spawn_batch_transforms(&mut self, transforms: Vec<TransformComponent>) -> Vec<Entity> {
        let entities: Vec<Entity> = self
            .world
            .spawn_batch(transforms)
            .collect();
        entities
    }

    /// Total active user entity count currently living in the Bevy ECS world.
    pub fn entity_count(&self) -> usize {
        (self.world.entities().len().saturating_sub(1)) as usize
    }
}
