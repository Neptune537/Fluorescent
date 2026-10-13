use std::sync::Arc;
use tokio::sync::mpsc;
use std::sync::RwLock;

/// Represents a high-level ECS draw command pushed to the render queue.
#[derive(Clone, Debug)]
pub struct RenderCommand {
    pub mesh_id: u32,
    pub material_id: u16,
    pub position: [f32; 3],
    pub sphere_radius: f32,
    pub rotation_quat: [f32; 4],
    pub scale: [f32; 3],
}

/// Unified GPU Command Buffer architecture.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PackedEntityInstance {
    pub position: [f32; 3],          // 12 bytes (Sphere center)
    pub sphere_radius: f32,          // 4 bytes  (Culling radius)
    pub rotation_quat: [f32; 4],     // 16 bytes (Orientation)
    pub scale: [f32; 3],             // 12 bytes (Non-uniform scale)
    pub cluster_and_flags: u32,      // 4 bytes  (16-bit cluster_id/flags | 16-bit material_id)
}

// Compile-time assertion to guarantee the exact 48-byte memory layout
const _: () = assert!(std::mem::size_of::<PackedEntityInstance>() == 48);

/// Type alias for backward compatibility.
pub type EntityInstance = PackedEntityInstance;

#[cfg(feature = "fluoridian")]
impl From<fluoridian_core::fluoderpod_render::packed_instance::PackedEntityInstance> for PackedEntityInstance {
    fn from(f: fluoridian_core::fluoderpod_render::packed_instance::PackedEntityInstance) -> Self {
        Self {
            position: f.position,
            sphere_radius: 1.0,
            rotation_quat: f.orientation,
            scale: [1.0, 1.0, 1.0],
            cluster_and_flags: ((f.entity_type as u32) << 16) | (f.flags as u32),
        }
    }
}
pub struct PipelineManager {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub entity_buffer: Option<wgpu::Buffer>,
    pub indirect_draw_buffer: Option<wgpu::Buffer>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,

    // Channel for ECS to push commands
    pub render_command_tx: mpsc::UnboundedSender<RenderCommand>,
    // Staging buffer to hold batched instances before GPU upload
    pub staging_instances: Arc<RwLock<Vec<PackedEntityInstance>>>,
}

impl PipelineManager {
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn init_from_openxr_vulkan(
        _vk_instance: ash::Instance,
        _vk_physical_device: ash::vk::PhysicalDevice,
        _vk_device: ash::Device,
        _queue_family_index: u32,
        _queue_index: u32,
    ) -> Result<Self, &'static str> {
        // For wgpu 0.20, creating a device from raw Vulkan handles requires wgpu_core.
        // Since wgpu_core is not directly exposed for this operation without specific features,
        // we return an Error instead of unimplemented!() to gracefully fail and allow the engine to fallback
        // or handle the error, preventing a hard crash.
        Err("Cannot directly construct wgpu::Device from wgpu_hal::Device in wgpu 0.20 without wgpu_core access.")
    }

    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        entity_buffer: Option<wgpu::Buffer>,
        indirect_draw_buffer: Option<wgpu::Buffer>,
        render_pipeline: Option<wgpu::RenderPipeline>,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<RenderCommand>();
        let staging_instances = Arc::new(RwLock::new(Vec::new()));
        let staging_instances_clone = staging_instances.clone();

        let task = async move {
            let mut pending_commands = Vec::new();
            while let Some(cmd) = rx.recv().await {
                pending_commands.push(cmd);

                // Drain any other immediately available commands
                while let Ok(cmd) = rx.try_recv() {
                    pending_commands.push(cmd);
                }

                // Sort by material ID to batch draw calls and minimize state changes
                pending_commands.sort_by_key(|c| c.material_id);

                // Compress RenderCommand -> PackedEntityInstance
                let mut instances = Vec::with_capacity(pending_commands.len());
                for cmd in &pending_commands {
                    instances.push(PackedEntityInstance {
                        position: cmd.position,
                        sphere_radius: cmd.sphere_radius,
                        rotation_quat: cmd.rotation_quat,
                        scale: cmd.scale,
                        // High 16 bits = cluster/flags (0 for now), Low 16 bits = material_id
                        cluster_and_flags: (cmd.material_id as u32),
                    });
                }

                // Write to staging buffer, which will be consumed by the main thread's execute_frame
                {
                    let mut lock = staging_instances_clone.write().unwrap();
                    lock.clear();
                    lock.extend(instances);
                }

                pending_commands.clear();
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(task);

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);

        Self {
            device,
            queue,
            entity_buffer,
            indirect_draw_buffer,
            render_pipeline,
            render_command_tx: tx,
            staging_instances,
        }
    }

    /// Allocates or updates the GPU buffer receiving the `fluoderpod` batch.
    pub fn update_entity_buffer(&mut self, data: &[u8]) {
        if let Some(ref buffer) = self.entity_buffer {
            self.queue.write_buffer(buffer, 0, data);
        }
    }

    /// Pulls from the staging buffer and uploads to the GPU.
    pub fn flush_staging_to_gpu(&mut self) {
        if let Some(ref buffer) = self.entity_buffer {
            let lock = self.staging_instances.read().unwrap();
            if !lock.is_empty() {
                let data = bytemuck::cast_slice(&*lock);
                self.queue.write_buffer(buffer, 0, data);
            }
        }
    }

    /// Submits the final indirect draw commands generated by the compute passes.
    /// Uses `multi_draw_indexed_indirect` if supported by the device, otherwise loops over `draw_indexed_indirect`
    /// to support multiple meshlets and instances.
    pub fn submit_indirect_draws<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, draw_count: u32) {
        if let (Some(pipeline), Some(indirect_buf)) = (&self.render_pipeline, &self.indirect_draw_buffer) {
            render_pass.set_pipeline(pipeline);

            if self.device.features().contains(wgpu::Features::MULTI_DRAW_INDIRECT) {
                // Maximum performance path: Single API call for all meshlets/instances
                render_pass.multi_draw_indexed_indirect(indirect_buf, 0, draw_count);
            } else {
                // Fallback path: Loop and dispatch individually
                // Note: The indirect draw args size is typically 20 bytes (5 * 4 bytes for DrawIndexedIndirectArgs)
                let stride = std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>() as u64;
                for i in 0..draw_count {
                    let offset = i as u64 * stride;
                    render_pass.draw_indexed_indirect(indirect_buf, offset);
                }
            }
        }
    }
}
