use crate::ecs::world::{EngineWorld, TransformComponent, PhysicsComponent};
use crate::ecs::components::geo::Player;
use serde::{Deserialize, Serialize};

pub trait MeshNode: Send + Sync {
    fn broadcast(&self, data: &[u8]);
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntityStateUpdate {
    pub entity_id: u64,
    pub timestamp: f64,
    pub transform: Option<TransformComponent>,
    pub physics: Option<PhysicsComponent>,
    pub velocity: Option<[f32; 3]>,
}

#[derive(Clone, Debug)]
pub struct NetworkInterpolationComponent {
    pub start_transform: TransformComponent,
    pub target_transform: TransformComponent,
    pub velocity: Option<[f32; 3]>,
    pub start_time: f64,
    pub target_time: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReplicationMessage {
    FullSync(Vec<EntityStateUpdate>),
    DeltaUpdate(Vec<EntityStateUpdate>),
}

pub struct ReplicationSystem {
    pub mesh_node: Box<dyn MeshNode>,
}

impl ReplicationSystem {
    pub fn new(mesh_node: Box<dyn MeshNode>) -> Self {
        Self { mesh_node }
    }

    pub fn broadcast_world_state(&self, world: &mut EngineWorld) {
        let mut viewer_position = [0.0, 0.0, 0.0];
        let visible_radius = 500.0;
        let radius_sq = visible_radius * visible_radius;

        // Find the local player's position to use as the culling origin
        for (_entity, (transform, _player)) in world.ecs.query::<(&TransformComponent, &Player)>().iter() {
            viewer_position[0] = transform.matrix[12];
            viewer_position[1] = transform.matrix[13];
            viewer_position[2] = transform.matrix[14];
            break; // Just grab the first player found
        }

        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        let mut updates = Vec::new();

        for (entity, (transform, phys)) in world.ecs.query::<(&TransformComponent, &PhysicsComponent)>().iter() {
            let pos_x = transform.matrix[12];
            let pos_y = transform.matrix[13];
            let pos_z = transform.matrix[14];

            let dx = pos_x - viewer_position[0];
            let dy = pos_y - viewer_position[1];
            let dz = pos_z - viewer_position[2];

            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq <= radius_sq {
                updates.push(EntityStateUpdate {
                    entity_id: entity.to_bits(),
                    timestamp: current_time,
                    transform: Some(*transform),
                    physics: Some(*phys),
                    velocity: None,
                });
            }
        }

        let message = ReplicationMessage::FullSync(updates);
        if let Ok(data) = bincode::serialize(&message) {
            self.mesh_node.broadcast(&data);
        }
    }

    pub fn receive_update(&self, world: &mut EngineWorld, data: &[u8]) {
        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        if let Ok(message) = bincode::deserialize::<ReplicationMessage>(data) {
            match message {
                ReplicationMessage::FullSync(updates) | ReplicationMessage::DeltaUpdate(updates) => {
                    for update in updates {
                        let entity = hecs::Entity::from_bits(update.entity_id);

                        // Apply transform if present
                        if let Some(t) = update.transform {
                            // We need to clone it because we can't hold a reference while calling insert/get_mut later
                            let start_transform = if let Ok(current_transform) = world.ecs.get::<TransformComponent>(entity) {
                                *current_transform
                            } else {
                                t
                            };

                            let mut has_interp = false;

                            if let Ok(mut interp) = world.ecs.get_mut::<NetworkInterpolationComponent>(entity) {
                                interp.start_transform = start_transform;
                                interp.target_transform = t;
                                interp.velocity = update.velocity;
                                interp.start_time = current_time;
                                // In a robust system, we would calculate server-to-client time delta offset
                                // For now, we simulate a 100ms interpolation window from the current client time
                                interp.target_time = current_time + 0.1;
                                has_interp = true;
                            }

                            if !has_interp {
                                if world.ecs.contains(entity) {
                                    let _ = world.ecs.insert_one(entity, NetworkInterpolationComponent {
                                        start_transform,
                                        target_transform: t,
                                        velocity: update.velocity,
                                        start_time: current_time,
                                        target_time: current_time + 0.1,
                                    });
                                }
                            }

                            // Since apply_network_interpolation is currently not hooked up to a main game loop,
                            // we must directly apply the transform here to prevent entities from freezing.
                            if let Ok(mut current_transform) = world.ecs.get_mut::<TransformComponent>(entity) {
                                *current_transform = t;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn apply_network_interpolation(world: &mut EngineWorld) {
        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        for (_entity, (transform, interp)) in world.ecs.query_mut::<(&mut TransformComponent, &NetworkInterpolationComponent)>() {
            if interp.target_time <= interp.start_time {
                *transform = interp.target_transform;
                continue;
            }

            let t = (current_time - interp.start_time) / (interp.target_time - interp.start_time);

            if t <= 1.0 {
                let t_clamped = t.clamp(0.0, 1.0) as f32;

                let mut new_matrix = interp.start_transform.matrix;

                let start_pos = [
                    interp.start_transform.matrix[12],
                    interp.start_transform.matrix[13],
                    interp.start_transform.matrix[14]
                ];

                let target_pos = [
                    interp.target_transform.matrix[12],
                    interp.target_transform.matrix[13],
                    interp.target_transform.matrix[14]
                ];

                new_matrix[12] = start_pos[0] + (target_pos[0] - start_pos[0]) * t_clamped;
                new_matrix[13] = start_pos[1] + (target_pos[1] - start_pos[1]) * t_clamped;
                new_matrix[14] = start_pos[2] + (target_pos[2] - start_pos[2]) * t_clamped;

                transform.matrix = new_matrix;
            } else {
                if let Some(vel) = interp.velocity {
                    let extrapolation_time = (current_time - interp.target_time) as f32;
                    let max_extrapolation = 0.5;
                    let time_to_extrapolate = extrapolation_time.min(max_extrapolation);

                    let mut new_matrix = interp.target_transform.matrix;

                    let target_pos = [
                        interp.target_transform.matrix[12],
                        interp.target_transform.matrix[13],
                        interp.target_transform.matrix[14]
                    ];

                    new_matrix[12] = target_pos[0] + vel[0] * time_to_extrapolate;
                    new_matrix[13] = target_pos[1] + vel[1] * time_to_extrapolate;
                    new_matrix[14] = target_pos[2] + vel[2] * time_to_extrapolate;

                    transform.matrix = new_matrix;
                } else {
                    *transform = interp.target_transform;
                }
            }
        }
    }
}
