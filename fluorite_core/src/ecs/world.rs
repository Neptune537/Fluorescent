use crate::physics::sync::sync_body_transform_to_mat4;
use crate::physics::world::PhysicsWorld;
use hecs::Entity;
use hecs::World as HecsWorld;
use rapier3d::prelude::RigidBodyHandle;
use serde::{Deserialize, Serialize};
use crate::streaming::world_streamer::ChunkCoord;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum StreamingCommand {
    SpawnEntities(ChunkCoord, usize),
    DespawnEntities(ChunkCoord),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TransformComponent {
    pub matrix: [f32; 16],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PhysicsComponent {
    #[serde(skip)]
    pub handle: Option<RigidBodyHandle>,
}

pub struct EngineWorld {
    pub ecs: HecsWorld,
    pub physics: PhysicsWorld,
    command_receiver: Receiver<StreamingCommand>,
    command_sender: Sender<StreamingCommand>,

    // Simulates mapping chunks to a block of spawned entities in the ECS
    pub resident_chunk_entities: std::collections::HashMap<ChunkCoord, Vec<Entity>>,
}

impl Default for EngineWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineWorld {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = channel();
        Self {
            ecs: HecsWorld::new(),
            physics: PhysicsWorld::new(),
            command_receiver,
            command_sender,
            resident_chunk_entities: std::collections::HashMap::new(),
        }
    }

    pub fn command_sender(&self) -> Sender<StreamingCommand> {
        self.command_sender.clone()
    }

    /// Process commands queued from background streaming threads.
    /// This avoids stutter by running these batch operations on the main thread safely.
    pub fn process_streaming_commands(&mut self) {
        while let Ok(cmd) = self.command_receiver.try_recv() {
            match cmd {
                StreamingCommand::SpawnEntities(coord, count) => {
                    // For massive streaming, we'd spawn batches of entities here
                    // e.g. self.ecs.spawn_batch(...)
                    // Simplified: We'll spawn empty entities and store them to track residency
                    let mut entities = Vec::with_capacity(count);
                    for _ in 0..count {
                        entities.push(self.ecs.spawn(())); // Spawn empty entity to simulate load
                    }
                    self.resident_chunk_entities.insert(coord, entities);
                }
                StreamingCommand::DespawnEntities(coord) => {
                    if let Some(entities) = self.resident_chunk_entities.remove(&coord) {
                        for entity in entities {
                            let _ = self.ecs.despawn(entity);
                        }
                    }
                }
            }
        }
    }

    pub fn sync_physics_to_ecs(&mut self) {
        let mut query = self
            .ecs
            .query::<(&PhysicsComponent, &mut TransformComponent)>();
        for (phys, transform) in query.iter() {
            if let Some(body) = self.physics.get_rigid_body(phys.handle.unwrap()) {
                transform.matrix = sync_body_transform_to_mat4(body);
            }
        }
    }
}
