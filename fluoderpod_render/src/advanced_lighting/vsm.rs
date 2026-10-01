use std::sync::Arc;

pub struct VirtualShadowMapConfig {
    pub resolution_per_page: u32,
    pub max_pages: u32,
}

pub struct VirtualShadowMapSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: VirtualShadowMapConfig,
}

impl VirtualShadowMapSystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: VirtualShadowMapConfig) -> Self {
        Self { device, queue, config }
    }

    pub fn execute(&self, _encoder: &mut wgpu::CommandEncoder) {
        // TODO: VSM rendering logic
    }
}
