//! Subsurface Scattering (SSS) BSSRDF Profiles for Skin, Eyes, and Translucent Media.
//!
//! Provides Burley normalized diffusion profiles and separable Gaussian kernels for screen-space
//! subsurface scattering (SSSSS).

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Subsurface scattering material parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct SubsurfaceProfile {
    /// Scattering color mean free path (RGB in millimeters)
    pub scatter_color: [f32; 3],
    /// Translucency scale factor
    pub scatter_distance: f32,
    /// Dual specular lobe roughness weights [primary, secondary]
    pub roughness_weights: [f32; 2],
    /// Curvature bias for thin geometric areas (ears, nostrils)
    pub curvature_bias: f32,
    /// Transmission thickness factor
    pub transmission_thickness: f32,
}

impl Default for SubsurfaceProfile {
    fn default() -> Self {
        // Default Caucasian skin profile based on Jensen/Burley diffusion measurements
        Self {
            scatter_color: [1.0, 0.45, 0.25],
            scatter_distance: 3.5, // 3.5mm mean free path
            roughness_weights: [0.35, 0.70],
            curvature_bias: 0.1,
            transmission_thickness: 1.0,
        }
    }
}

impl SubsurfaceProfile {
    /// Creates a customized realistic human eye cornea/sclera profile.
    pub fn eye_profile() -> Self {
        Self {
            scatter_color: [0.85, 0.88, 0.92],
            scatter_distance: 1.2,
            roughness_weights: [0.05, 0.25],
            curvature_bias: 0.05,
            transmission_thickness: 0.8,
        }
    }

    /// Evaluates Burley normalized diffusion profile $R(r)$ for radial distance $r$.
    pub fn evaluate_burley_diffusion(&self, r: f32) -> Vec3 {
        let d = Vec3::from_slice(&self.scatter_color) * self.scatter_distance.max(1e-4);
        let s = Vec3::splat(1.9) - d + 3.5 * (-6.0 * (d - Vec3::splat(0.8)).abs()).exp();

        let l = Vec3::splat(r) / (d + Vec3::splat(1e-4));
        let r_val = (-l * s).exp() + (-l * s / 3.0).exp();
        r_val / (Vec3::splat(8.0 * std::f32::consts::PI) * d)
    }
}
