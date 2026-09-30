//! Hardware-Accelerated Ray-Traced Global Illumination (RTGI) and Irradiance Field.
//!
//! Provides Acceleration Structure bindings (TLAS/BLAS) and real-time probe irradiance caching
//! for infinite-bounce indirect diffuse lighting.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Top-Level Acceleration Structure (TLAS) Instance descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TlasInstance {
    pub transform: [[f32; 4]; 3],            // 3x4 row-major transform
    pub instance_custom_index_and_mask: u32, // 24-bit custom index, 8-bit mask
    pub instance_shader_binding_table_record_offset_and_flags: u32,
    pub acceleration_structure_reference: u64, // 64-bit GPU device address to BLAS
}

const _: () = assert!(std::mem::size_of::<TlasInstance>() == 64);

/// Configuration for Ray-Traced Global Illumination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtgiConfig {
    pub rays_per_pixel: u32,
    pub max_bounces: u32,
    pub temporal_accumulation_factor: f32,
    pub normal_rejection_threshold: f32,
    pub depth_rejection_threshold: f32,
    pub probe_grid_dimensions: [u32; 3],
    pub probe_spacing: f32,
}

impl Default for RtgiConfig {
    fn default() -> Self {
        Self {
            rays_per_pixel: 1,
            max_bounces: 2,
            temporal_accumulation_factor: 0.95,
            normal_rejection_threshold: 0.8,
            depth_rejection_threshold: 0.05,
            probe_grid_dimensions: [32, 16, 32],
            probe_spacing: 1.5,
        }
    }
}

/// Dynamic Irradiance Probe in 3D world space.
#[derive(Debug, Clone, Copy)]
pub struct IrradianceProbe {
    pub position: Vec3,
    /// L0 and L1 Spherical Harmonics coefficients (RGB irradiance)
    pub sh_coefficients: [[f32; 3]; 4],
}

impl IrradianceProbe {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            sh_coefficients: [[0.0; 3]; 4],
        }
    }

    /// Evaluates irradiance in a given surface normal direction using SH reconstruction.
    pub fn evaluate_irradiance(&self, normal: Vec3) -> Vec3 {
        // C0 = 0.282095 (L0)
        // C1 = 0.488603 (L1)
        let c0 = 0.282095;
        let c1 = 0.488603;

        let mut irr = Vec3::from_slice(&self.sh_coefficients[0]) * c0;
        irr += Vec3::from_slice(&self.sh_coefficients[1]) * (c1 * normal.y);
        irr += Vec3::from_slice(&self.sh_coefficients[2]) * (c1 * normal.z);
        irr += Vec3::from_slice(&self.sh_coefficients[3]) * (c1 * normal.x);

        irr.max(Vec3::ZERO)
    }
}

/// RTGI Pipeline Controller.
pub struct RtgiPipeline {
    pub config: RtgiConfig,
    pub probes: Vec<IrradianceProbe>,
}

impl RtgiPipeline {
    pub fn new(config: RtgiConfig) -> Self {
        let (dim_x, dim_y, dim_z) = (
            config.probe_grid_dimensions[0],
            config.probe_grid_dimensions[1],
            config.probe_grid_dimensions[2],
        );

        let mut probes = Vec::with_capacity((dim_x * dim_y * dim_z) as usize);
        let spacing = config.probe_spacing;

        for z in 0..dim_z {
            for y in 0..dim_y {
                for x in 0..dim_x {
                    let pos = Vec3::new(
                        (x as f32 - dim_x as f32 * 0.5) * spacing,
                        (y as f32) * spacing,
                        (z as f32 - dim_z as f32 * 0.5) * spacing,
                    );
                    probes.push(IrradianceProbe::new(pos));
                }
            }
        }

        Self { config, probes }
    }

    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }
}
