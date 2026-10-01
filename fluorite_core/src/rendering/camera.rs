//! 3D Camera Projection Matrices and Uniform Management for PBR Pipelines.
//!
//! Provides perspective, orthographic, reverse-Z depth projections, frustum extraction,
//! and conversion into WebGPU/Vulkan std140-aligned `CameraUniforms`.

use crate::rendering::pbr::CameraUniforms;
use crate::spatial::Frustum;
use glam::{Mat4, Vec3};

/// Camera projection mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionType {
    Perspective {
        fov_y_radians: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        z_near: f32,
        z_far: f32,
    },
}

/// 3D Camera holding world-space position, orientation, and projection settings.
#[derive(Debug, Clone)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection_type: ProjectionType,
    pub reverse_z: bool,
}

impl Camera3D {
    /// Creates a default perspective camera looking at the origin from (0, 5, 10).
    pub fn new_perspective(
        position: Vec3,
        target: Vec3,
        aspect_ratio: f32,
        fov_degrees: f32,
    ) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            projection_type: ProjectionType::Perspective {
                fov_y_radians: fov_degrees.to_radians(),
                aspect_ratio,
                z_near: 0.1,
                z_far: 1000.0,
            },
            reverse_z: true, // Recommended for IEEE 754 float depth precision
        }
    }

    /// Creates an orthographic camera.
    pub fn new_orthographic(
        position: Vec3,
        target: Vec3,
        width: f32,
        height: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        Self {
            position,
            target,
            up: Vec3::Y,
            projection_type: ProjectionType::Orthographic {
                left: -half_w,
                right: half_w,
                bottom: -half_h,
                top: half_h,
                z_near: near,
                z_far: far,
            },
            reverse_z: false,
        }
    }

    /// Computes the View Matrix: $V = \text{look\_at}(eye, target, up)$.
    pub fn compute_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Computes the Projection Matrix ($P$) with Vulkan/WebGPU clip-space depth range $[0.0, 1.0]$.
    pub fn compute_projection_matrix(&self) -> Mat4 {
        match self.projection_type {
            ProjectionType::Perspective {
                fov_y_radians,
                aspect_ratio,
                z_near,
                z_far,
            } => {
                if self.reverse_z {
                    // Infinite reverse-Z perspective projection for zero Z-fighting
                    Mat4::perspective_infinite_reverse_rh(fov_y_radians, aspect_ratio, z_near)
                } else {
                    Mat4::perspective_rh(fov_y_radians, aspect_ratio, z_near, z_far)
                }
            }
            ProjectionType::Orthographic {
                left,
                right,
                bottom,
                top,
                z_near,
                z_far,
            } => Mat4::orthographic_rh(left, right, bottom, top, z_near, z_far),
        }
    }

    /// Computes the combined View-Projection Matrix: $VP = P \times V$.
    pub fn compute_view_projection(&self) -> (Mat4, Mat4, Mat4) {
        let view = self.compute_view_matrix();
        let proj = self.compute_projection_matrix();
        let view_proj = proj * view;
        (view, proj, view_proj)
    }

    /// Extracts the 6 view-frustum clipping planes from the combined View-Projection matrix.
    pub fn extract_frustum(&self) -> Frustum {
        let (_, _, view_proj) = self.compute_view_projection();
        Frustum::from_view_projection(view_proj)
    }

    /// Assembles the complete 320-byte `CameraUniforms` struct ready for GPU upload.
    pub fn to_uniforms(
        &self,
        screen_width: f32,
        screen_height: f32,
        cluster_dims: (u32, u32, u32),
        num_dynamic_lights: u32,
        ambient_light: [f32; 4],
    ) -> CameraUniforms {
        let (view, proj, view_proj) = self.compute_view_projection();
        let inv_proj = proj.inverse();

        let (z_near, z_far) = match self.projection_type {
            ProjectionType::Perspective { z_near, z_far, .. } => (z_near, z_far),
            ProjectionType::Orthographic { z_near, z_far, .. } => (z_near, z_far),
        };

        CameraUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            inv_proj: inv_proj.to_cols_array_2d(),
            camera_pos: self.position.to_array(),
            z_near,
            z_far,
            screen_width,
            screen_height,
            cluster_dim_x: cluster_dims.0,
            cluster_dim_y: cluster_dims.1,
            cluster_dim_z: cluster_dims.2,
            num_dynamic_lights,
            _padding: 0,
            ambient_light,
        }
    }

    /// Translates camera in local camera coordinates (e.g. WASD movement).
    pub fn move_local(&mut self, delta_right: f32, delta_up: f32, delta_forward: f32) {
        let forward = (self.target - self.position).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let actual_up = right.cross(forward).normalize_or_zero();

        let translation = right * delta_right + actual_up * delta_up + forward * delta_forward;
        self.position += translation;
        self.target += translation;
    }

    /// Orbits around target by given azimuth and elevation (in radians).
    pub fn orbit(&mut self, azimuth: f32, elevation: f32) {
        let offset = self.position - self.target;
        let radius = offset.length();
        if radius < 1e-4 {
            return;
        }

        let mut current_elevation = (offset.y / radius).asin();
        let mut current_azimuth = offset.z.atan2(offset.x);

        current_azimuth += azimuth;
        current_elevation = (current_elevation + elevation).clamp(-1.5, 1.5);

        let new_offset = Vec3::new(
            radius * current_elevation.cos() * current_azimuth.cos(),
            radius * current_elevation.sin(),
            radius * current_elevation.cos() * current_azimuth.sin(),
        );

        self.position = self.target + new_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_matrices() {
        let camera =
            Camera3D::new_perspective(Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO, 16.0 / 9.0, 60.0);

        let (view, proj, view_proj) = camera.compute_view_projection();
        assert!(!view.is_nan());
        assert!(!proj.is_nan());
        assert!(!view_proj.is_nan());

        let uniforms = camera.to_uniforms(1920.0, 1080.0, (16, 9, 24), 8, [0.1, 0.1, 0.1, 1.0]);
        assert_eq!(uniforms.screen_width, 1920.0);
        assert_eq!(uniforms.screen_height, 1080.0);
        assert_eq!(uniforms.cluster_dim_x, 16);
    }
}
