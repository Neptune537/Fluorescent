//! High-Throughput Line-Of-Sight (LOS) Raycasting.
//!
//! Performs batched visibility tests across thousands/millions of rays against spatial BVHs.

use crate::spatial::{FlatBvh, Ray};
use glam::Vec3;

/// Single Line-Of-Sight query between an observer and a target entity.
#[derive(Debug, Clone, Copy)]
pub struct LosBatchQuery {
    pub observer_id: u32,
    pub target_id: u32,
    pub origin: Vec3,
    pub target: Vec3,
    pub max_view_distance: f32,
    pub max_fov_cos: f32, // Cosine of half FOV angle
    pub observer_forward: Vec3,
}

/// Result of an individual LOS test.
#[derive(Debug, Clone, Copy)]
pub struct LosBatchResult {
    pub observer_id: u32,
    pub target_id: u32,
    pub has_los: bool,
    pub hit_distance: f32,
}

pub struct LosChecker;

impl LosChecker {
    /// Evaluates visibility for a batch of queries against a static scene BVH.
    pub fn evaluate_batch(queries: &[LosBatchQuery], bvh: &FlatBvh) -> Vec<LosBatchResult> {
        let mut results = Vec::with_capacity(queries.len());

        for q in queries {
            let to_target = q.target - q.origin;
            let dist = to_target.length();

            // 1. Distance rejection
            if dist > q.max_view_distance || dist < 1e-4 {
                results.push(LosBatchResult {
                    observer_id: q.observer_id,
                    target_id: q.target_id,
                    has_los: false,
                    hit_distance: f32::INFINITY,
                });
                continue;
            }

            let dir = to_target / dist;

            // 2. Field-of-view (FOV) cone rejection
            let fov_cos = q.observer_forward.dot(dir);
            if fov_cos < q.max_fov_cos {
                results.push(LosBatchResult {
                    observer_id: q.observer_id,
                    target_id: q.target_id,
                    has_los: false,
                    hit_distance: f32::INFINITY,
                });
                continue;
            }

            // 3. BVH Occlusion Raycast
            let ray = Ray::new(q.origin.into(), dir.into(), 0.0, dist);
            let hit = bvh.raycast(&ray, &[]);

            let has_los = match hit {
                Some(h) => h.distance >= dist - 0.05, // Hit beyond or at target
                None => true,                         // Clear unobstructed sightline
            };

            results.push(LosBatchResult {
                observer_id: q.observer_id,
                target_id: q.target_id,
                has_los,
                hit_distance: hit.map(|h| h.distance).unwrap_or(dist),
            });
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_los_clear() {
        let bvh = FlatBvh::default();
        let query = LosBatchQuery {
            observer_id: 1,
            target_id: 2,
            origin: Vec3::ZERO,
            target: Vec3::new(0.0, 0.0, 10.0),
            max_view_distance: 50.0,
            max_fov_cos: 0.5,
            observer_forward: Vec3::new(0.0, 0.0, 1.0),
        };

        let res = LosChecker::evaluate_batch(&[query], &bvh);
        assert_eq!(res.len(), 1);
        assert!(res[0].has_los);
    }
}

