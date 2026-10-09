//! Handles runtime physics-driven destruction, chunk separation, and rigid body spawning.

use crate::ecs::world::{EngineWorld, PhysicsComponent, TransformComponent};
use hecs::Entity;
use rand::Rng;
use rapier3d::na::Point3;
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

        // 2. Extract compound shapes before despawning the original entity.
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
            // Perform Voronoi 3D cellular fracture on the entity's mesh at the `impact_point`.
            let num_seeds = 12;
            let mut rng = rand::thread_rng();
            let mut seeds = Vec::with_capacity(num_seeds);

            for _ in 0..num_seeds {
                seeds.push(glam::Vec3::new(
                    rng.gen_range(-0.5..0.5),
                    rng.gen_range(-0.5..0.5),
                    rng.gen_range(-0.5..0.5),
                ));
            }

            let grid_size = 10;
            let step = 1.0 / (grid_size as f32 - 1.0);

            let mut cell_points: Vec<Vec<glam::Vec3>> = vec![Vec::new(); num_seeds];

            for x in 0..grid_size {
                for y in 0..grid_size {
                    for z in 0..grid_size {
                        let p = glam::Vec3::new(
                            -0.5 + (x as f32) * step,
                            -0.5 + (y as f32) * step,
                            -0.5 + (z as f32) * step,
                        );

                        let mut min_dist = f32::MAX;
                        let mut closest_seed = 0;

                        for (i, seed) in seeds.iter().enumerate() {
                            let dist = (p - *seed).length_squared();
                            if dist < min_dist {
                                min_dist = dist;
                                closest_seed = i;
                            }
                        }

                        cell_points[closest_seed].push(p);
                    }
                }
            }

            for points in cell_points {
                if points.is_empty() {
                    continue;
                }

                let mut centroid = glam::Vec3::ZERO;
                for p in &points {
                    centroid += *p;
                }
                centroid /= points.len() as f32;

                let mut hull_points = Vec::with_capacity(points.len());
                for p in &points {
                    let rel_p = *p - centroid;
                    hull_points.push(Point3::new(rel_p.x, rel_p.y, rel_p.z));
                }

                let chunk_pos = base_pos + centroid;

                // Spawn new child entities for each fractured chunk, instantiating dynamic Rapier3D rigid bodies.
                let mut body = RigidBodyBuilder::dynamic()
                    .translation(vector![chunk_pos.x, chunk_pos.y, chunk_pos.z])
                    .build();

                // Apply outward impulse forces to the chunks to simulate the explosion/impact.
                let dir = (chunk_pos - impact_point).normalize_or_zero();
                let impulse = dir * (force * 0.1); // Scaled force distribution
                body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);

                let body_handle = world.physics.insert_rigid_body(body);

                // Create a small collider for the chunk
                let collider = ColliderBuilder::convex_hull(&hull_points)
                    .unwrap_or_else(|| ColliderBuilder::cuboid(0.1, 0.1, 0.1))
                    .build();
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
}
