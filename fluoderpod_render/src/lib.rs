pub mod culling;
#[cfg(all(not(target_arch = "wasm32"), feature = "nexus"))]
pub mod nexus_client;
#[cfg(all(not(target_arch = "wasm32"), feature = "reticulum"))]
pub mod reticulum_bridge;
pub mod unified_pipeline;
pub mod virtual_geometry;
pub mod thermal;
pub mod entity_packet;
pub mod geofencing;

#[cfg(feature = "fluoridian")]
pub mod fluoridian_bridge {
    pub use fluoridian_core;
    pub use fluoridian_core::fluoderpod::cesium::*;
    pub use fluoridian_core::fluoderpod_render::packed_instance::PackedEntityInstance as FluoridianPackedInstance;
}

/// The main entry point for the fluoderpod GPU-driven render graph.
pub struct FluoderpodRenderer {
    // The unified pipeline manager handles wgpu device/queue
    pub pipeline_manager: unified_pipeline::PipelineManager,
    // Add systems to manage dispatch
    pub culling_system: Option<culling::ComputeCuller>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "reticulum"))]
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

    pub vsm_system: Option<advanced_lighting::vsm::VirtualShadowMapSystem>,
    pub gi_system: Option<advanced_lighting::gi::DynamicGISystem>,

    pub num_instances: u32,
    pub num_clusters: u32,
    pub thermal_scaler: thermal::ThermalScaler,
    pub virtual_geometry_streamer: Option<virtual_geometry::ClusterStreamer>,
    pub max_page_faults: u32,
    
    pub geofence_registry: geofencing::GeofenceRegistry,
    pub aircraft_telemetry: std::collections::HashMap<String, unified_pipeline::PackedEntityInstance>,
    pub viewport_mode: virtual_geometry::globe::ViewportMode,
    pub ring_tile_manager: virtual_geometry::globe::RingTileManager,
    pub camera_ecef: [f64; 3],
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
            #[cfg(all(not(target_arch = "wasm32"), feature = "reticulum"))]
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
            vsm_system: None,
            gi_system: None,
            num_instances: 0,
            num_clusters: 0,
            thermal_scaler: thermal::ThermalScaler::new(),
            virtual_geometry_streamer: None,
            max_page_faults,
            geofence_registry: geofencing::GeofenceRegistry::new(),
            aircraft_telemetry: std::collections::HashMap::new(),
            viewport_mode: virtual_geometry::globe::ViewportMode::Globe,
            ring_tile_manager: virtual_geometry::globe::RingTileManager::new(virtual_geometry::globe::LocalTileRingConfig::default()),
            camera_ecef: [virtual_geometry::globe::EARTH_RADIUS_WGS84 + 10_000.0, 0.0, 0.0],
        }
    }

    /// Ingests a raw contiguous buffer of 48-byte PackedEntityInstance structs directly from FFI.
    /// This bypasses CPU-side iteration and prepares the data for compute culling.
    pub fn push_render_command(&mut self, cmd: unified_pipeline::RenderCommand) {
        let _ = self.pipeline_manager.render_command_tx.send(cmd);
    }

    /// Ingests a contiguous buffer of 64-byte little-endian `EntityPacket64` wire packets
    /// (see `ENTITY_PACKET_CONTRACT.md`), decoding them into GPU instances.
    /// Never panics; a length not divisible by 64 returns `Err`.
    pub fn ingest_entity_packets(&mut self, raw: &[u8]) -> Result<usize, entity_packet::EntityPacketError> {
        let instances = entity_packet::decode_to_gpu_instances(raw)?;
        self.ingest_fluoderpod_batch(bytemuck::cast_slice(&instances))
            .map_err(|_| entity_packet::EntityPacketError::InvalidLength { len: raw.len() })
    }

    pub fn ingest_fluoderpod_batch(&mut self, raw_entity_data: &[u8]) -> Result<usize, &'static str> {
        if raw_entity_data.len() % std::mem::size_of::<unified_pipeline::PackedEntityInstance>() != 0 {
            return Err("Entity buffer byte length is not a multiple of 48 bytes");
        }

        // Validate bytemuck cast without copying
        let mut instances: Vec<unified_pipeline::PackedEntityInstance> = bytemuck::try_cast_slice(raw_entity_data)
            .map_err(|_| "Failed to cast entity slice to PackedEntityInstance")?
            .to_vec();

        let initial_count = instances.len();
        instances.extend(self.aircraft_telemetry.values().cloned());
        
        let instance_count = instances.len();
        self.num_instances = instance_count as u32;

        #[cfg(all(not(target_arch = "wasm32"), feature = "reticulum"))]
        if let Some(bridge) = &self.reticulum_bridge {
            bridge.send_nexus_message(reticulum_bridge::NexusMessage::Telemetry {
                payload: format!("Batch update with {} instances ({} aircraft)", instance_count, instance_count - initial_count)
            });
        }

        // High-throughput upload to GPU buffers handled by the unified pipeline
        self.pipeline_manager.update_entity_buffer(bytemuck::cast_slice(&instances));

        Ok(instance_count)
    }

    /// Executes pre-render compute passes (culling, virtual geometry evaluation, VSM)
    pub fn dispatch_compute_passes(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.pipeline_manager.flush_staging_to_gpu();

        // Process incoming mesh network messages (e.g. MapUpdates from the editor/network)
        #[cfg(all(not(target_arch = "wasm32"), feature = "reticulum"))]
        if let Some(bridge) = &self.reticulum_bridge {
            while let Some(msg) = bridge.try_recv_message() {
                match msg {
                    reticulum_bridge::NexusMessage::MapUpdate { json_payload } => {
                        if let Ok(new_registry) = geofencing::GeofenceRegistry::from_json(&json_payload) {
                            self.geofence_registry = new_registry;
                            log::info!("Geofence map updated via Reticulum");
                        }
                    }
                    reticulum_bridge::NexusMessage::LocationUpdate { entity_id, lat, lng, timestamp: _, flight } => {
                        // Calculate ECEF for flight visualization (assume ~10,000m cruising alt if unknown)
                        let alt = 10000.0;
                        let a = 6378137.0; // WGS84 major axis
                        let e_sq = 0.00669437999014;
                        let lat_rad = lat.to_radians();
                        let lon_rad = lng.to_radians();
                        let n = a / (1.0 - e_sq * lat_rad.sin() * lat_rad.sin()).sqrt();
                        
                        let x = (n + alt) * lat_rad.cos() * lon_rad.cos();
                        let y = (n + alt) * lat_rad.cos() * lon_rad.sin();
                        let z = (n * (1.0 - e_sq) + alt) * lat_rad.sin();
                        let entity_pos = [x, y, z];
                        // Telemetry Gating: If in LocalMap mode, only store entities within the local neighborhood bubble
                        let should_ingest = match self.viewport_mode {
                            virtual_geometry::globe::ViewportMode::Globe => true,
                            virtual_geometry::globe::ViewportMode::LocalMap => {
                                self.ring_tile_manager.is_entity_in_telemetry_bubble(self.camera_ecef, entity_pos)
                            }
                        };

                        if should_ingest {
                            let mut packet = crate::entity_packet::EntityPacket64 {
                                x, y, z,
                                entity_type: crate::entity_packet::ENTITY_TYPE_AIRCRAFT,
                                status: 1,
                                faction: 0,
                                bounty_le: [0; 8],
                                id: [0; 20],
                            };
                            
                            // Use flight name or entity_id as id
                            let ident = flight.unwrap_or(entity_id.clone());
                            let bytes = ident.as_bytes();
                            for i in 0..bytes.len().min(20) {
                                packet.id[i] = bytes[i];
                            }

                            let packed = crate::unified_pipeline::PackedEntityInstance {
                                position: [x as f32, y as f32, z as f32],
                                sphere_radius: 20.0,
                                rotation_quat: [0.0, 0.0, 0.0, 1.0],
                                scale: [1.0, 1.0, 1.0],
                                cluster_and_flags: packet.entity_type,
                            };
                            
                            self.aircraft_telemetry.insert(ident, packed);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 0. Dispatch Pre-Passes: VSM and SSGI
        if let Some(vsm) = &self.vsm_system {
            // Integrate AstronomyEngine for directional lighting
            // Assuming we have a cached camera lat/lng and timestamp, or we just calculate it from origin
            // For now, use a default lat/lng or extract from geofence/camera
            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
            let bodies = crate::virtual_geometry::globe::AstronomyEngine::compute_celestial_positions(37.7749, -122.4194, timestamp);
            
            // Extract sun direction
            let mut sun_dir = [0.0, -1.0, 0.0];
            if let Some(sun) = bodies.iter().find(|b| b.name == "Sun") {
                let az = sun.azimuth;
                let el = sun.elevation;
                // Convert spherical azimuth/elevation to cartesian direction vector
                sun_dir = [
                    (el.cos() * az.sin()) as f32,
                    el.sin() as f32,
                    (el.cos() * az.cos()) as f32,
                ];
            }

            vsm.execute(encoder, sun_dir);
        }

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
    }

    /// Submits unified indirect draw commands into an active render pass.
    pub fn submit_indirect_draws<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.pipeline_manager.submit_indirect_draws(render_pass, self.num_instances);
    }

    /// Executes the full GPU-driven frame onto a target texture view:
    /// Pre-Passes -> Compute Culling -> Virtual Geometry LOD -> Indirect Draw -> Thermal Throttle
    pub fn render_frame_to_view(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.dispatch_compute_passes(encoder);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.submit_indirect_draws(&mut render_pass);
        }

        self.thermal_scaler.throttle();
    }

    /// Executes the full GPU-driven frame: Culling -> Virtual Geometry LOD -> Indirect Draw
    pub fn execute_frame<'a>(&'a mut self, encoder: &mut wgpu::CommandEncoder, render_pass: &mut wgpu::RenderPass<'a>) {
        self.thermal_scaler.throttle();
        self.dispatch_compute_passes(encoder);
        self.submit_indirect_draws(render_pass);
    }

    /// Evaluates logic per tick (e.g. checking boundary crosses against the geofence registry).
    /// Returns the IDs of the active geofences the player currently resides in.
    pub fn update_logic(&mut self, player_position: [f32; 3]) -> Vec<geofencing::GeofenceId> {
        let active_zones = self.geofence_registry.query_point(player_position);
        
        // Optionally, emit a telemetry event if the active zone changes.
        // We'll leave the diffing to the caller (Flutter/Dart) to minimize Rust state tracking.
        active_zones
    }

    /// Update the thermal scaler's activity state based on engine input (e.g. camera movement, entity updates)
    pub fn set_scene_activity(&mut self, is_static: bool) {
        self.thermal_scaler.update_state(is_static);
    }
}
#[cfg(target_os = "android")]
pub mod android_vulkan;

#[cfg(target_arch = "wasm32")]
pub mod web_canvas;
#[cfg(target_arch = "wasm32")]
pub use web_canvas::*;

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
    // Wire format: 64-byte EntityPacket64 (see ENTITY_PACKET_CONTRACT.md).
    // Returns entity count (>= 0), -1 null pointer, -2 invalid length.
    match renderer.ingest_entity_packets(data_slice) {
        Ok(count) => i32::try_from(count).unwrap_or(i32::MAX),
        Err(_) => -2,
    }
}

/// Convenience FFI for platforms where the renderer is held globally (e.g. Android JNI or Web).
/// Copies 64-byte `EntityPacket64` structs directly into the active renderer buffers.
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_ingest_batch_global(
    data_ptr: *const u8,
    data_len: usize,
) -> i32 {
    if data_ptr.is_null() {
        return -1;
    }
    let data_slice = std::slice::from_raw_parts(data_ptr, data_len);
    #[cfg(target_os = "android")]
    {
        android_vulkan::ingest_into_any_active_session(data_slice)
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_canvas::fluoderpod_ingest_batch_web(data_slice)
    }
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        let _ = data_slice;
        -1
    }
}

/// Alias for global batch ingestion
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_ingest_batch_ptr(
    data_ptr: *const u8,
    data_len: usize,
) -> i32 {
    fluoderpod_ingest_batch_global(data_ptr, data_len)
}

/// FFI for pre-built 48-byte GPU `PackedEntityInstance` buffers.
/// Returns instance count, -1 null pointer, -2 invalid length.
#[no_mangle]
pub unsafe extern "C" fn fluoderpod_ingest_gpu_instances(
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
        Ok(count) => i32::try_from(count).unwrap_or(i32::MAX),
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

#[cfg(feature = "xr")]
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

#[cfg(all(not(target_arch = "wasm32"), feature = "webrtc-stream"))]
pub mod webrtc_signalling;
#[cfg(all(not(target_arch = "wasm32"), feature = "webrtc-stream"))]
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
