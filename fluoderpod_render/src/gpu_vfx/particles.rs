use std::sync::Arc;

pub struct ParticleConfig {
    pub max_particles: u32,
    pub emit_rate: f32,
}

pub struct ParticleEmitter {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color: [f32; 4],
    pub active: bool,
}

pub struct GpuParticleSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: ParticleConfig,
    // compute_pipeline: wgpu::ComputePipeline,
    // render_pipeline: wgpu::RenderPipeline,
}

impl GpuParticleSystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: ParticleConfig) -> Self {
        Self { device, queue, config }
    }

    pub fn simulate(&self, _encoder: &mut wgpu::CommandEncoder, _delta_time: f32) {
        // TODO: Dispatch compute shader for particle updates
    }

    pub fn render(&self, _pass: &mut wgpu::RenderPass<'_>) {
        // TODO: Render particles
    }
}
