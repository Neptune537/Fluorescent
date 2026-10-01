use std::sync::Arc;

pub struct GIConfig {
    pub ray_steps: u32,
    pub intensity: f32,
    pub fallback_probe_intensity: f32,
}

impl Default for GIConfig {
    fn default() -> Self {
        Self {
            ray_steps: 16,
            intensity: 1.0,
            fallback_probe_intensity: 0.5,
        }
    }
}

pub struct DynamicGISystem {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub config: GIConfig,
    pub ssgi_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl DynamicGISystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: GIConfig) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSGI Bind Group Layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSGI Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSGI Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ssgi.wgsl").into()),
        });

        let ssgi_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SSGI Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Self {
            device,
            queue,
            config,
            ssgi_pipeline,
            bind_group_layout,
        }
    }

    pub fn compute(&self, encoder: &mut wgpu::CommandEncoder, bind_group: &wgpu::BindGroup, width: u32, height: u32) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("SSGI Compute Pass"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.ssgi_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
    }
}
