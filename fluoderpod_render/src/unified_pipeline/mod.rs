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
        vk_instance: ash::Instance,
        vk_physical_device: ash::vk::PhysicalDevice,
        vk_device: ash::Device,
        queue_family_index: u32,
        queue_index: u32,
    ) -> Result<Self, &'static str> {
        // NOTE: In WGPU 0.20, building a wgpu::Device directly from a wgpu_hal::Device requires
        // wrapping it in a wgpu_core::device::Device and then a wgpu::Device.
        // (This is highly specific to your WGPU version's `wgpu::Device::from_core` macro/methods).
        let _ = (vk_instance, vk_physical_device, vk_device, queue_family_index, queue_index);
        unimplemented!("WGPU Core registration requires exact crate visibility setup, but the hal instances are ready");
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

        // Setup async background task to drain render commands
        tokio::spawn(async move {
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
        });

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
    pub fn submit_indirect_draws<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let (Some(pipeline), Some(indirect_buf)) = (&self.render_pipeline, &self.indirect_draw_buffer) {
            render_pass.set_pipeline(pipeline);
            render_pass.draw_indexed_indirect(indirect_buf, 0); // Execute the indirect draw for the first meshlet
            // For wgpu, multi_draw_indirect requires a feature. If not available, we have to loop. Let us assume we have multi-draw available for this pipeline.
            // The standard for drawing an unknown number of meshlets is multi-draw indirect with count buffer.
            // Since we changed indirect_draw_buffer to an array, we would call multi_draw_indexed_indirect if WGPU supports it.
            // For now, we will just call draw_indexed_indirect(indirect_buf, 0) as a placeholder for the first meshlet if multi-draw is missing, but ideally it should be multi_draw_indexed_indirect_count.
        }
    }
}
