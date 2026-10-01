//! Cascaded Shadow Maps (CSM) with 4-Cascade Practical Splits and Texel Snapping.
//!
//! Eliminates shadow edge shimmering and swimming during camera rotation by computing
//! tight spherical bounds and snapping light-space projection matrices to shadow map texel intervals.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

pub const NUM_CASCADES: usize = 4;

/// CSM Cascade Configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmConfig {
    pub num_cascades: usize,
    pub split_lambda: f32, // Blend factor between uniform (0.0) and logarithmic (1.0) split
    pub shadow_map_resolution: u32,
    pub max_shadow_distance: f32,
}

impl Default for CsmConfig {
    fn default() -> Self {
        Self {
            num_cascades: NUM_CASCADES,
            split_lambda: 0.85,
            shadow_map_resolution: 2048,
            max_shadow_distance: 300.0,
        }
    }
}

/// Computed cascade data.
#[derive(Debug, Clone)]
pub struct CascadeData {
    pub split_depth: f32,
    pub view_proj: Mat4,
    pub sphere_center: Vec3,
    pub sphere_radius: f32,
}

/// Cascaded Shadow Mapping Calculator.
pub struct CascadedShadowMap {
    pub config: CsmConfig,
}

impl CascadedShadowMap {
    pub fn new(config: CsmConfig) -> Self {
        Self { config }
    }

    /// Computes cascade split distances using the practical split scheme:
    /// $C_i = \lambda \cdot C_{log} + (1 - \lambda) \cdot C_{uniform}$.
    pub fn compute_split_distances(&self, z_near: f32) -> [f32; NUM_CASCADES] {
        let mut splits = [0.0; NUM_CASCADES];
        let n = z_near;
        let f = self.config.max_shadow_distance;
        let lambda = self.config.split_lambda;

        for i in 0..NUM_CASCADES {
            let p = (i + 1) as f32 / NUM_CASCADES as f32;
            let log_split = n * (f / n).powf(p);
            let uniform_split = n + (f - n) * p;
            splits[i] = lambda * log_split + (1.0 - lambda) * uniform_split;
        }

        splits
    }

    /// Computes stable, snapped light-space View-Projection matrices for all cascades.
    pub fn compute_cascades(
        &self,
        camera_view: &Mat4,
        fov_y: f32,
        aspect_ratio: f32,
        z_near: f32,
        light_direction: Vec3,
    ) -> [CascadeData; NUM_CASCADES] {
        let splits = self.compute_split_distances(z_near);
        let light_dir = light_direction.normalize();
        let inv_cam_view = camera_view.inverse();

        let mut cascades = Vec::with_capacity(NUM_CASCADES);
        let mut prev_split = z_near;

        for &split in &splits {
            // Frustum corners in camera space
            let half_v_fov = fov_y * 0.5;
            let tan_half_v_fov = half_v_fov.tan();
            let tan_half_h_fov = tan_half_v_fov * aspect_ratio;

            let x_near = prev_split * tan_half_h_fov;
            let y_near = prev_split * tan_half_v_fov;
            let x_far = split * tan_half_h_fov;
            let y_far = split * tan_half_v_fov;

            let corners_cam = [
                Vec3::new(-x_near, -y_near, -prev_split),
                Vec3::new( x_near, -y_near, -prev_split),
                Vec3::new( x_near,  y_near, -prev_split),
                Vec3::new(-x_near,  y_near, -prev_split),
                Vec3::new(-x_far,  -y_far,  -split),
                Vec3::new( x_far,  -y_far,  -split),
                Vec3::new( x_far,   y_far,  -split),
                Vec3::new(-x_far,   y_far,  -split),
            ];

            // Transform corners to world space and compute bounding sphere
            let mut center = Vec3::ZERO;
            let mut corners_world = [Vec3::ZERO; 8];
            for (idx, c) in corners_cam.iter().enumerate() {
                let w = inv_cam_view.transform_point3(*c);
                corners_world[idx] = w;
                center += w;
            }
            center /= 8.0;

            let mut radius: f32 = 0.0;
            for w in &corners_world {
                radius = radius.max((*w - center).length());
            }

            // Texel snapping: round bounding sphere center to shadow map texel grid units
            let texels_per_unit = self.config.shadow_map_resolution as f32 / (radius * 2.0);
            let light_view = Mat4::look_to_rh(center, light_dir, Vec3::Y);

            let mut light_space_center = light_view.transform_point3(center);
            light_space_center.x = (light_space_center.x * texels_per_unit).floor() / texels_per_unit;
            light_space_center.y = (light_space_center.y * texels_per_unit).floor() / texels_per_unit;

            let snapped_center = light_view.inverse().transform_point3(light_space_center);
            let light_view_snapped = Mat4::look_to_rh(snapped_center, light_dir, Vec3::Y);

            let light_proj = Mat4::orthographic_rh(
                -radius, radius,
                -radius, radius,
                -radius * 4.0, radius * 4.0,
            );

            cascades.push(CascadeData {
                split_depth: split,
                view_proj: light_proj * light_view_snapped,
                sphere_center: snapped_center,
                sphere_radius: radius,
            });

            prev_split = split;
        }

        [
            cascades[0].clone(),
            cascades[1].clone(),
            cascades[2].clone(),
            cascades[3].clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csm_splits() {
        let csm = CascadedShadowMap::new(CsmConfig::default());
        let splits = csm.compute_split_distances(0.1);
        assert!(splits[0] < splits[1]);
        assert!(splits[1] < splits[2]);
        assert!(splits[2] < splits[3]);
        assert!((splits[3] - 300.0).abs() < 1.0);
    }
}
