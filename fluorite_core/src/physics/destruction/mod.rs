//! Handles runtime physics-driven destruction, chunk separation, and rigid body spawning.

use crate::ecs::world::{EngineWorld, PhysicsComponent, TransformComponent};
use hecs::Entity;
use rapier3d::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Destructible {
    pub impact_threshold: f32,
}

pub struct DestructionSystem;

impl DestructionSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_fracture(
        &mut self,
        world: &mut EngineWorld,
        entity: Entity,
        impact_point: glam::Vec3,
        force: f32,
    ) {
        // 1. Check if the entity is destructible and if the impact `_force` exceeds its threshold.
        let mut is_fractured = false;
        if let Ok(destructible) = world.ecs.query_one_mut::<&Destructible>(entity) {
            if force >= destructible.impact_threshold {
                is_fractured = true;
            }
        }

        if !is_fractured {
            return;
        }

        let mut base_pos = glam::Vec3::ZERO;
        let mut base_rotation = rapier3d::na::UnitQuaternion::identity();
        let mut old_handle = None;

        if let Ok(transform) = world.ecs.query_one_mut::<&TransformComponent>(entity) {
            base_pos = glam::Vec3::new(
                transform.matrix[12],
                transform.matrix[13],
                transform.matrix[14],
            );
        }

        if let Ok(phys) = world.ecs.query_one_mut::<&PhysicsComponent>(entity) {
            old_handle = phys.handle;
        }

        // 2. Extract compound shapes (Voronoi cells) before despawning the original entity.
        let mut fractured_parts = Vec::new();
        if let Some(handle) = old_handle {
            if let Some(body) = world.physics.get_rigid_body(handle) {
                base_rotation = *body.rotation();
                for collider_handle in body.colliders() {
                    if let Some(collider) = world.physics.get_collider(*collider_handle) {
                        if let Some(compound) = collider.shape().as_compound() {
                            for (isometry, shape) in compound.shapes() {
                                fractured_parts.push((isometry.clone(), shape.clone()));
                            }
                        }
                    }
                }
            }
        }

        // 3. Despawn the original entity.
        if let Some(handle) = old_handle {
            world.physics.remove_rigid_body(handle);
        }
        let _ = world.ecs.despawn(entity);

        // 4. Spawn new child entities for each fractured chunk, instantiating dynamic Rapier3D rigid bodies.
        if !fractured_parts.is_empty() {
            for (isometry, shape) in fractured_parts {
                let local_offset = rapier3d::na::Vector3::new(
                    isometry.translation.vector.x,
                    isometry.translation.vector.y,
                    isometry.translation.vector.z,
                );
                // Apply parent rotation to local offset
                let rotated_offset = base_rotation * local_offset;

                let chunk_pos = base_pos + glam::Vec3::new(rotated_offset.x, rotated_offset.y, rotated_offset.z);
                let chunk_rot = base_rotation * isometry.rotation;

                let mut body = RigidBodyBuilder::dynamic()
                    .translation(vector![chunk_pos.x, chunk_pos.y, chunk_pos.z])
                    .rotation(chunk_rot.scaled_axis())
                    .build();

                // 5. Apply outward impulse forces to the chunks to simulate the explosion/impact.
                let dir = (chunk_pos - impact_point).normalize_or_zero();
                let impulse = dir * (force * 0.1);
                body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);

                let body_handle = world.physics.insert_rigid_body(body);

                let chunk_collider = ColliderBuilder::new(shape).build();
                world.physics.insert_collider(chunk_collider, body_handle);

                let matrix = crate::physics::sync::sync_body_transform_to_mat4(
                    world.physics.get_rigid_body(body_handle).unwrap()
                );

                world.ecs.spawn((
                    TransformComponent { matrix },
                    PhysicsComponent {
                        handle: Some(body_handle),
                    },
                ));
            }
        } else {
            // Mock fallback if the entity didn't have a compound shape.
            let num_chunks = 8;
            for i in 0..num_chunks {
                let angle = (i as f32) * std::f32::consts::PI / 4.0;
                let offset = glam::Vec3::new(angle.cos(), 0.5, angle.sin()) * 1.0;
                let chunk_pos = base_pos + offset;

                let mut body = RigidBodyBuilder::dynamic()
                    .translation(vector![chunk_pos.x, chunk_pos.y, chunk_pos.z])
                    .build();

                let dir = (chunk_pos - impact_point).normalize_or_zero();
                let impulse = dir * (force * 0.1);
                body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);

                let body_handle = world.physics.insert_rigid_body(body);

                let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5).build();
                world.physics.insert_collider(collider, body_handle);

                let mut matrix = [0.0; 16];
                matrix[0] = 1.0;
                matrix[5] = 1.0;
                matrix[10] = 1.0;
                matrix[15] = 1.0;
                matrix[12] = chunk_pos.x;
                matrix[13] = chunk_pos.y;
                matrix[14] = chunk_pos.z;

                world.ecs.spawn((
                    TransformComponent { matrix },
                    PhysicsComponent {
                        handle: Some(body_handle),
                    },
                ));
            }
        }
    }
}
