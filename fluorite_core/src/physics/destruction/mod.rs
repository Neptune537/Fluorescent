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

        // 3. Despawn the original entity.
        if let Some(handle) = old_handle {
            world.physics.remove_rigid_body(handle);
        }
        let _ = world.ecs.despawn(entity);

        // 2. Perform Voronoi 3D cellular fracture on the entity's mesh at the `_impact_point`.
        // (Mock implementation: spawning 8 chunks)
        let num_chunks = 8;
        for i in 0..num_chunks {
            // Offset logic for mock chunks
            let angle = (i as f32) * std::f32::consts::PI / 4.0;
            let offset = glam::Vec3::new(angle.cos(), 0.5, angle.sin()) * 1.0;
            let chunk_pos = base_pos + offset;

            // 4. Spawn new child entities for each fractured chunk, instantiating dynamic Rapier3D rigid bodies.
            let mut body = RigidBodyBuilder::dynamic()
                .translation(vector![chunk_pos.x, chunk_pos.y, chunk_pos.z])
                .build();

            // 5. Apply outward impulse forces to the chunks to simulate the explosion/impact.
            let dir = (chunk_pos - impact_point).normalize_or_zero();
            let impulse = dir * (force * 0.1); // Scaled force distribution
            body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);

            let body_handle = world.physics.insert_rigid_body(body);

            // Create a small collider for the chunk
            let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5).build();
            world.physics.insert_collider(collider, body_handle);

            // Identity matrix with position embedded
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
