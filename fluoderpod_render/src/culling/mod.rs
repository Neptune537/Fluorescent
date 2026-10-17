use wgpu;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// Camera Uniforms structure matching the WGSL definitions,
/// used for GPU-driven frustum and occlusion culling.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],       // 64 bytes
    pub camera_pos: [f32; 3],           // 12 bytes
    pub _pad0: f32,                     // 4 bytes (align to 16)
    pub frustum_planes: [[f32; 4]; 6],  // 96 bytes: 6 planes (Left, Right, Bottom, Top, Near, Far)
    pub viewport_size: [f32; 2],        // 8 bytes
    pub _pad1: [f32; 2],                // 8 bytes
}

/// Dynamic impact deformation zone for WPO conservative bounding expansion.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ImpactDeformationZone {
    pub center: [f32; 3],
    pub radius: f32,
    pub max_displacement: f32,
    pub _pad: [f32; 3],
}

/// Uniforms defining the active dynamic impact zones count.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ImpactUniforms {
    pub zone_count: u32,
    pub _pad: [u32; 3],
}

/// ComputeCuller structure.
pub struct ComputeCuller {
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,

    // Pipelines
    frustum_pipeline: wgpu::ComputePipeline,
    hzb_downsample_pipeline: wgpu::ComputePipeline,
    hzb_culling_pipeline: wgpu::ComputePipeline,

    // Bind group layouts
    frustum_bind_group_layout: wgpu::BindGroupLayout,
    hzb_downsample_bind_group_layout: wgpu::BindGroupLayout,
    hzb_culling_bind_group_layout: wgpu::BindGroupLayout,

    // Default deformation buffers for standard culling
    pub default_impact_zones_buffer: wgpu::Buffer,
    pub default_impact_uniforms_buffer: wgpu::Buffer,
}

impl ComputeCuller {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Frustum pipeline
        let frustum_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fluoderpod Frustum Culling Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("frustum_culling.wgsl").into()),
        });

        let frustum_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Frustum Culling Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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

        let frustum_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Frustum Culling Pipeline Layout"),
            bind_group_layouts: &[&frustum_bind_group_layout],
            push_constant_ranges: &[],
        });

        let frustum_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Frustum Culling Pipeline"),
            layout: Some(&frustum_pipeline_layout),
            module: &frustum_shader,
            compilation_options: Default::default(),
            entry_point: "main",
        });

        // HZB Downsample pipeline
        let hzb_downsample_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fluoderpod HZB Downsample Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hzb_downsample.wgsl").into()),
        });

        let hzb_downsample_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HZB Downsample Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::R32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let hzb_downsample_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HZB Downsample Pipeline Layout"),
            bind_group_layouts: &[&hzb_downsample_bind_group_layout],
            push_constant_ranges: &[],
        });

        let hzb_downsample_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("HZB Downsample Pipeline"),
            layout: Some(&hzb_downsample_pipeline_layout),
            module: &hzb_downsample_shader,
            compilation_options: Default::default(),
            entry_point: "main",
        });

        // HZB Occlusion Culling pipeline
        let hzb_culling_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fluoderpod HZB Occlusion Culling Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hzb_culling.wgsl").into()),
        });

        let hzb_culling_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HZB Occlusion Culling Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let hzb_culling_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HZB Occlusion Culling Pipeline Layout"),
            bind_group_layouts: &[&hzb_culling_bind_group_layout],
            push_constant_ranges: &[],
        });

        let hzb_culling_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("HZB Occlusion Culling Pipeline"),
            layout: Some(&hzb_culling_pipeline_layout),
            module: &hzb_culling_shader,
            compilation_options: Default::default(),
            entry_point: "main",
        });

        // Initialize default zero-impact buffers for standard culling
        let default_impact_zones_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Default Impact Zones Buffer"),
            size: std::mem::size_of::<ImpactDeformationZone>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let default_impact_uniforms = ImpactUniforms {
            zone_count: 0,
            _pad: [0; 3],
        };
        let default_impact_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Default Impact Uniforms Buffer"),
            size: std::mem::size_of::<ImpactUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&default_impact_uniforms_buffer, 0, bytemuck::bytes_of(&default_impact_uniforms));

        Self {
            device,
            queue,
            frustum_pipeline,
            hzb_downsample_pipeline,
            hzb_culling_pipeline,
            frustum_bind_group_layout,
            hzb_downsample_bind_group_layout,
            hzb_culling_bind_group_layout,
            default_impact_zones_buffer,
            default_impact_uniforms_buffer,
        }
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        unimplemented!("Mock implementation not provided yet")
    }

    /// Dispatch frustum culling compute shader.
    pub fn dispatch_frustum_culling(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera_uniform_buffer: &wgpu::Buffer,
        instances_buffer: &wgpu::Buffer,
        indirect_draw_buffer: &wgpu::Buffer,
        compact_instances_buffer: &wgpu::Buffer,
        num_instances: u32,
    ) {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frustum Culling Bind Group"),
            layout: &self.frustum_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instances_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indirect_draw_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: compact_instances_buffer.as_entire_binding(),
                },
            ],
        });

        encoder.clear_buffer(indirect_draw_buffer, 4, Some(4));
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Frustum Culling Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.frustum_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);

        let workgroups = (num_instances + 63) / 64; // Assuming 64 threads per workgroup
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Dispatch HZB Downsample compute shader.
    pub fn dispatch_hzb_downsample(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input_texture: &wgpu::TextureView,
        output_texture: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HZB Downsample Bind Group"),
            layout: &self.hzb_downsample_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(output_texture),
                },
            ],
        });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("HZB Downsample Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.hzb_downsample_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);

        let workgroups_x = (width + 7) / 8; // Assuming 8x8 workgroup size
        let workgroups_y = (height + 7) / 8;
        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    /// Dispatch hierarchical Z-buffer (HZB) occlusion culling.
    pub fn dispatch_occlusion_culling(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera_uniform_buffer: &wgpu::Buffer,
        instances_buffer: &wgpu::Buffer,
        indirect_draw_buffer: &wgpu::Buffer,
        compact_instances_buffer: &wgpu::Buffer,
        hzb_texture: &wgpu::TextureView,
        hzb_sampler: &wgpu::Sampler,
        num_instances: u32,
    ) {
        self.dispatch_occlusion_culling_with_deformation(
            encoder,
            camera_uniform_buffer,
            instances_buffer,
            indirect_draw_buffer,
            compact_instances_buffer,
            hzb_texture,
            hzb_sampler,
            &self.default_impact_zones_buffer,
            &self.default_impact_uniforms_buffer,
            num_instances,
        );
    }

    /// Dispatch hierarchical Z-buffer (HZB) occlusion culling with dynamic impact deformation zones.
    /// Instances/clusters within active impact zones will have their bounding radii conservatively expanded
    /// by max_displacement to prevent false-positive occlusion culling during WPO soft-body deformation.
    pub fn dispatch_occlusion_culling_with_deformation(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera_uniform_buffer: &wgpu::Buffer,
        instances_buffer: &wgpu::Buffer,
        indirect_draw_buffer: &wgpu::Buffer,
        compact_instances_buffer: &wgpu::Buffer,
        hzb_texture: &wgpu::TextureView,
        hzb_sampler: &wgpu::Sampler,
        impact_zones_buffer: &wgpu::Buffer,
        impact_uniforms_buffer: &wgpu::Buffer,
        num_instances: u32,
    ) {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HZB Occlusion Culling Bind Group"),
            layout: &self.hzb_culling_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instances_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indirect_draw_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: compact_instances_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(hzb_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(hzb_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: impact_zones_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: impact_uniforms_buffer.as_entire_binding(),
                },
            ],
        });

        encoder.clear_buffer(indirect_draw_buffer, 4, Some(4));
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("HZB Occlusion Culling Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.hzb_culling_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);

        let workgroups = (num_instances + 63) / 64; // Assuming 64 threads per workgroup
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_deformation_zone_layout() {
        assert_eq!(std::mem::size_of::<ImpactDeformationZone>(), 32);
        assert_eq!(std::mem::align_of::<ImpactDeformationZone>(), 4);
    }

    #[test]
    fn test_impact_uniforms_layout() {
        assert_eq!(std::mem::size_of::<ImpactUniforms>(), 16);
        assert_eq!(std::mem::align_of::<ImpactUniforms>(), 4);
    }

    #[test]
    fn test_conservative_expansion_calculation() {
        let instance_center = [10.0f32, 0.0, 0.0];
        let original_radius = 2.0f32;

        let zone = ImpactDeformationZone {
            center: [12.0, 0.0, 0.0],
            radius: 5.0,
            max_displacement: 1.5,
            _pad: [0.0; 3],
        };

        let dx = instance_center[0] - zone.center[0];
        let dy = instance_center[1] - zone.center[1];
        let dz = instance_center[2] - zone.center[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        let mut expanded_radius = original_radius;
        if distance < (original_radius + zone.radius) {
            expanded_radius += zone.max_displacement;
        }

        assert_eq!(expanded_radius, 3.5);
    }
}
