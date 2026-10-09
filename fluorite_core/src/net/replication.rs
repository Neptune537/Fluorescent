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

    pub fn get_visible_entities(world: &EngineWorld, viewer_position: [f32; 3], cull_radius_sq: f32) -> Vec<hecs::Entity> {
        let mut visible = Vec::new();
        for (entity, transform) in world.ecs.query_mut::<&TransformComponent>() {
            let pos_x = transform.matrix[12];
            let pos_y = transform.matrix[13];
            let pos_z = transform.matrix[14];

            let dx = pos_x - viewer_position[0];
            let dy = pos_y - viewer_position[1];
            let dz = pos_z - viewer_position[2];

            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq <= cull_radius_sq {
                visible.push(entity);
            }
        }
        visible
    }

    pub fn broadcast_world_state(&self, world: &mut EngineWorld) {
        let mut viewer_position = [0.0, 0.0, 0.0];
        let cull_radius = 500.0;
        let cull_radius_sq = cull_radius * cull_radius;

        // Find the local player's position to use as the culling origin
        for (_entity, (transform, _player)) in world.ecs.query_mut::<(&TransformComponent, &Player)>() {
            viewer_position[0] = transform.matrix[12];
            viewer_position[1] = transform.matrix[13];
            viewer_position[2] = transform.matrix[14];
            break; // Just grab the first player found
        }

        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        let mut updates = Vec::new();

        let visible_entities = Self::get_visible_entities(world, viewer_position, cull_radius_sq);

        for entity in visible_entities {
            if let Ok(mut transform_q) = world.ecs.query_one::<&TransformComponent>(entity) {
              if let Some(transform) = transform_q.get() {
                if let Ok(mut phys_q) = world.ecs.query_one::<&PhysicsComponent>(entity) {
                  if let Some(phys) = phys_q.get() {
                    updates.push(EntityStateUpdate {
                        entity_id: entity.to_bits().into(),
                        timestamp: current_time,
                        transform: Some(*transform),
                        physics: Some(*phys),
                        velocity: None,
                    });
                }
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
                            let mut q = world.ecs.query_one::<&TransformComponent>(entity).unwrap();
                            let start_transform = if let Some(current_transform) = q.get() {
                                *current_transform
                            } else {
                                t
                            };

                            let mut has_interp = false;

                            let mut q_interp = world.ecs.query_one::<&mut NetworkInterpolationComponent>(entity).unwrap();
                            if let Some(interp) = q_interp.get() {
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

                        }
                    }
                }
            }
        }
    }

    pub fn extract_network_interpolation(world: &mut EngineWorld) {
        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        for (_entity, (transform, interp)) in world.ecs.query_mut::<(&mut TransformComponent, &NetworkInterpolationComponent)>() {
            if interp.target_time <= interp.start_time {
                *transform = interp.target_transform;
                continue;
            }

            let delta_time = current_time - interp.start_time;
            let tick_rate = interp.target_time - interp.start_time;

            if delta_time <= tick_rate {
                // Interpolate
                let t = (delta_time / tick_rate).clamp(0.0, 1.0) as f32;
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

                new_matrix[12] = start_pos[0] + (target_pos[0] - start_pos[0]) * t;
                new_matrix[13] = start_pos[1] + (target_pos[1] - start_pos[1]) * t;
                new_matrix[14] = start_pos[2] + (target_pos[2] - start_pos[2]) * t;

                transform.matrix = new_matrix;
            } else {
                // Extrapolate
                if let Some(vel) = interp.velocity {
                    let extrapolation_time = (delta_time - tick_rate) as f32;
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
