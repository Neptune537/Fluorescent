use std::sync::Arc;

pub struct GIConfig {
    pub ray_steps: u32,
    pub intensity: f32,
}

pub struct DynamicGISystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: GIConfig,
}

impl DynamicGISystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: GIConfig) -> Self {
        Self { device, queue, config }
    }

    pub fn compute(&self, _encoder: &mut wgpu::CommandEncoder) {
        // TODO: GI compute pass
    }
}
