pub mod culling;
#[cfg(not(target_arch = "wasm32"))]
pub mod nexus_client;
#[cfg(not(target_arch = "wasm32"))]
pub mod reticulum_bridge;
pub mod unified_pipeline;
pub mod virtual_geometry;
pub mod thermal;

/// The main entry point for the fluoderpod GPU-driven render graph.
pub struct FluoderpodRenderer {
    // The unified pipeline manager handles wgpu device/queue
    pub pipeline_manager: unified_pipeline::PipelineManager,
    // Add systems to manage dispatch
    pub culling_system: Option<culling::ComputeCuller>,
    #[cfg(not(target_arch = "wasm32"))]
    pub reticulum_bridge: Option<std::sync::Arc<reticulum_bridge::ReticulumBridge>>,
    pub virtual_geometry_system: Option<virtual_geometry::VirtualGeometrySystem>,

    // Buffers needed for passes
    pub camera_uniform_buffer: Option<wgpu::Buffer>,
    pub instances_buffer: Option<wgpu::Buffer>,
    pub compact_instances_buffer: Option<wgpu::Buffer>,
    pub cluster_metadata_buffer: Option<wgpu::Buffer>,
    pub output_clusters_buffer: Option<wgpu::Buffer>,
    pub draw_count_buffer: Option<wgpu::Buffer>,
    pub page_fault_buffer: Option<wgpu::Buffer>,
    pub hzb_texture: Option<wgpu::TextureView>,
    pub hzb_sampler: Option<wgpu::Sampler>,

    pub num_instances: u32,
    pub num_clusters: u32,
    pub thermal_scaler: thermal::ThermalScaler,
    pub virtual_geometry_streamer: Option<virtual_geometry::ClusterStreamer>,
    pub max_page_faults: u32,
}

impl FluoderpodRenderer {
    pub fn new(pipeline_manager: unified_pipeline::PipelineManager) -> Self {
        let max_page_faults = 1024;
        let page_fault_buffer_size = 4 + max_page_faults as wgpu::BufferAddress * 4;

        let page_fault_buffer = pipeline_manager.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Page Fault Buffer"),
            size: page_fault_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with count 0
        let zero: u32 = 0;
        pipeline_manager.queue.write_buffer(&page_fault_buffer, 0, bytemuck::bytes_of(&zero));

        Self {
            pipeline_manager,
            culling_system: None,
            #[cfg(not(target_arch = "wasm32"))]
            reticulum_bridge: None,
            virtual_geometry_system: None,
            camera_uniform_buffer: None,
            instances_buffer: None,
            compact_instances_buffer: None,
            cluster_metadata_buffer: None,
            output_clusters_buffer: None,
            draw_count_buffer: None,
            page_fault_buffer: Some(page_fault_buffer),
            hzb_texture: None,
            hzb_sampler: None,
            num_instances: 0,
            num_clusters: 0,
            thermal_scaler: thermal::ThermalScaler::new(),
            virtual_geometry_streamer: None,
            max_page_faults,
        }
    }

    /// Ingests a raw contiguous buffer of 48-byte PackedEntityInstance structs directly from FFI.
    /// This bypasses CPU-side iteration and prepares the data for compute culling.
    pub fn push_render_command(&mut self, cmd: unified_pipeline::RenderCommand) {
        let _ = self.pipeline_manager.render_command_tx.send(cmd);
    }

    pub fn ingest_fluoderpod_batch(&mut self, raw_entity_data: &[u8]) -> Result<usize, &'static str> {
        if raw_entity_data.len() % std::mem::size_of::<unified_pipeline::PackedEntityInstance>() != 0 {
            return Err("Entity buffer byte length is not a multiple of 48 bytes");
        }

        // Validate bytemuck cast without copying
        let _instances: &[unified_pipeline::PackedEntityInstance] = bytemuck::try_cast_slice(raw_entity_data)
            .map_err(|_| "Failed to cast entity slice to PackedEntityInstance")?;

        let instance_count = _instances.len();
        self.num_instances = instance_count as u32;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(bridge) = &self.reticulum_bridge {
            bridge.send_nexus_message(reticulum_bridge::NexusMessage::Telemetry {
                payload: format!("Batch update with {} instances", instance_count)
            });
        }

        // High-throughput upload to GPU buffers handled by the unified pipeline
        self.pipeline_manager.update_entity_buffer(raw_entity_data);

        Ok(instance_count)
    }

    /// Executes the full GPU-driven frame: Culling -> Virtual Geometry LOD -> Indirect Draw
    pub fn execute_frame<'a>(&'a mut self, encoder: &mut wgpu::CommandEncoder, render_pass: &mut wgpu::RenderPass<'a>) {
        self.pipeline_manager.flush_staging_to_gpu();

        // 1. Dispatch Frustum & Occlusion Culling
        if let (Some(culler), Some(camera_buf), Some(instances_buf), Some(indirect_buf), Some(compact_buf), Some(hzb_tex), Some(hzb_samp)) = (
            &self.culling_system,
            &self.camera_uniform_buffer,
            &self.instances_buffer,
            &self.pipeline_manager.indirect_draw_buffer,
            &self.compact_instances_buffer,
            &self.hzb_texture,
            &self.hzb_sampler
        ) {
            culler.dispatch_occlusion_culling(
                encoder,
                camera_buf,
                instances_buf,
                indirect_buf,
                compact_buf,
                hzb_tex,
                hzb_samp,
                self.num_instances
            );
        } else if let (Some(culler), Some(camera_buf), Some(instances_buf), Some(indirect_buf), Some(compact_buf)) = (
            &self.culling_system,
            &self.camera_uniform_buffer,
            &self.instances_buffer,
            &self.pipeline_manager.indirect_draw_buffer,
            &self.compact_instances_buffer
        ) {
            // Fallback to frustum culling if HZB is not available
            culler.dispatch_frustum_culling(
                encoder,
                camera_buf,
                instances_buf,
                indirect_buf,
                compact_buf,
                self.num_instances
            );
        }

        // 2. Dispatch Virtual Geometry Cluster Selection
        if let (Some(vg_system), Some(camera_buf), Some(metadata_buf), Some(output_buf), Some(draw_count_buf), Some(page_fault_buf)) = (
            &self.virtual_geometry_system,
            &self.camera_uniform_buffer,
            &self.cluster_metadata_buffer,
            &self.output_clusters_buffer,
            &self.draw_count_buffer,
            &self.page_fault_buffer
        ) {
            vg_system.dispatch_cluster_evaluation(
                encoder,
                camera_buf,
                metadata_buf,
                output_buf,
                draw_count_buf,
                page_fault_buf,
                self.num_clusters,
                &self.pipeline_manager.device
            );
        }

        // 3. Issue Unified Indirect Draw commands
        self.pipeline_manager.submit_indirect_draws(render_pass);

        // 4. Readback page faults for virtual geometry streaming
        if let (Some(streamer), Some(page_fault_buf)) = (
            &self.virtual_geometry_streamer,
            &self.page_fault_buffer
        ) {
            streamer.readback_page_faults(
                &self.pipeline_manager.device,
                &self.pipeline_manager.queue,
                page_fault_buf,
                self.max_page_faults,
            );
        }

        // 5. Adaptive Thermal Throttling
        self.thermal_scaler.throttle();
    }

    /// Update the thermal scaler's activity state based on engine input (e.g. camera movement, entity updates)
    pub fn set_scene_activity(&mut self, is_static: bool) {
        self.thermal_scaler.update_state(is_static);
    }
}
#[cfg(target_os = "android")]
pub mod android_vulkan;

#[cfg(target_arch = "wasm32")]
pub async fn create_webgpu_surface_from_canvas(
    instance: &wgpu::Instance,
    canvas_id: &str,
) -> Result<wgpu::Surface<'static>, Box<dyn std::error::Error>> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("No global window found")?;
    let document = window.document().ok_or("No document found")?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| format!("Canvas element '{}' not found", canvas_id))?;
    let html_canvas: web_sys::HtmlCanvasElement = canvas.dyn_into().map_err(|_| "Element is not a canvas")?;
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(html_canvas))?;
    Ok(surface)
}

/// C-ABI FFI function for Flutter to pass byte pointers directly via dart:ffi
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_ingest_batch(
    renderer_ptr: *mut FluoderpodRenderer,
    data_ptr: *const u8,
    data_len: usize,
) -> i32 {
    if renderer_ptr.is_null() || data_ptr.is_null() {
        return -1;
    }
    let renderer = &mut *renderer_ptr;
    let data_slice = std::slice::from_raw_parts(data_ptr, data_len);
    match renderer.ingest_fluoderpod_batch(data_slice) {
        Ok(count) => count as i32,
        Err(_) => -2,
    }
}

/// FFI to inform the renderer if the scene is static
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_set_scene_activity(
    renderer_ptr: *mut FluoderpodRenderer,
    is_static: bool,
) {
    if !renderer_ptr.is_null() {
        let renderer = &mut *renderer_ptr;
        renderer.set_scene_activity(is_static);
    }
}

pub mod xr;

/// FFI to start the OpenXR VR Session
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_start_vr_session() -> i32 {
    // In a full implementation, we'd spawn a background thread running the VR render loop
    // using the renderer instance.
    log::info!("VR session started via FFI");
    1
}

/// FFI to stop the OpenXR VR Session
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_stop_vr_session() -> i32 {
    log::info!("VR session stopped via FFI");
    1
}

#[cfg(not(target_arch = "wasm32"))]
pub mod webrtc_signalling;
#[cfg(not(target_arch = "wasm32"))]
pub mod webrtc_streamer;

pub mod headless_render;

#[cfg(not(target_arch = "wasm32"))]
pub mod video_encoder;
pub mod advanced_lighting;
pub mod gpu_vfx;

#[cfg(test)]
mod tests {
    use super::*;
    use unified_pipeline::PackedEntityInstance;

    #[test]
    fn test_packed_entity_instance_layout_48_bytes() {
        assert_eq!(std::mem::size_of::<PackedEntityInstance>(), 48);
        assert_eq!(std::mem::align_of::<PackedEntityInstance>(), 4);

        let instance = PackedEntityInstance {
            position: [1.0, 2.0, 3.0],
            sphere_radius: 5.0,
            rotation_quat: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            cluster_and_flags: 0x1234_5678,
        };

        let bytes = bytemuck::bytes_of(&instance);
        assert_eq!(bytes.len(), 48);

        let casted: &PackedEntityInstance = bytemuck::from_bytes(bytes);
        assert_eq!(*casted, instance);
    }

    #[test]
    fn test_batch_byte_length_validation() {
        let valid_single = vec![0u8; 48];
        let valid_double = vec![0u8; 96];
        let invalid_odd = vec![0u8; 24];
        let invalid_unaligned = vec![0u8; 50];

        assert_eq!(valid_single.len() % std::mem::size_of::<PackedEntityInstance>(), 0);
        assert_eq!(valid_double.len() % std::mem::size_of::<PackedEntityInstance>(), 0);
        assert_ne!(invalid_odd.len() % std::mem::size_of::<PackedEntityInstance>(), 0);
        assert_ne!(invalid_unaligned.len() % std::mem::size_of::<PackedEntityInstance>(), 0);
    }
}
