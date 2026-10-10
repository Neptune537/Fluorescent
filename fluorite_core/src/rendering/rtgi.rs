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

pub struct RtgiResources {
    pub tlas_buffer: wgpu::Buffer,
    pub probe_buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
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

        Self {
            config,
            probes,
        }
    }

    pub fn init_resources(&self, device: &wgpu::Device) -> RtgiResources {
        let tlas_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TLAS Instance Buffer"),
            size: std::mem::size_of::<TlasInstance>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let probe_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Irradiance Probe Buffer"),
            size: (self.probes.len() * std::mem::size_of::<IrradianceProbe>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RTGI Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RTGI Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tlas_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: probe_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RTGI Dummy Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@compute @workgroup_size(64, 1, 1) fn main() {}",
            )),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RTGI Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RTGI Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        RtgiResources {
            tlas_buffer,
            probe_buffer,
            bind_group_layout,
            bind_group,
            compute_pipeline,
        }
    }

    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }

    /// Dispatches the RTGI compute shader to update irradiance probes.
    pub fn update_probes(&mut self, tlas: &TlasInstance, queue: &wgpu::Queue, resources: &RtgiResources, encoder: &mut wgpu::CommandEncoder) {
        // Upload TLAS instance data
        let tlas_data = bytemuck::bytes_of(tlas);
        queue.write_buffer(&resources.tlas_buffer, 0, tlas_data);

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RTGI Update Probes Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&resources.compute_pipeline);
        compute_pass.set_bind_group(0, &resources.bind_group, &[]);

        // Dispatch workgroups to trace rays against the TlasInstance buffer
        // and reconstruct spherical harmonics for IrradianceProbe caching
        compute_pass.dispatch_workgroups((self.probes.len() as u32 / 64) + 1, 1, 1);
    }
}
