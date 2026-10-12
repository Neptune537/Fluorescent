use std::collections::HashMap;
use std::path::PathBuf;
pub mod globe;
pub use globe::{
    CubeFace, FloatingOriginCamera, GpuTerrainPatch, MultiScalePhase, MultiScaleState,
    PatchKey, QuadtreeGlobe, TileIndex, EARTH_RADIUS_WGS84,
};

/// Virtual Geometry (Nanite-style) cluster evaluation.

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ClusterHeader {
    pub sphere_bounds: [f32; 4],
    pub lod_error: f32,
    pub parent_lod_error: f32,
    pub cluster_id: u32,
    pub parent_cluster_id: u32,
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub _padding: [u32; 2],
}

#[derive(Clone, Debug)]
pub struct ClusterData {
    pub header: ClusterHeader,
    pub vertex_data: Vec<u8>,
    pub index_data: Vec<u8>,
}

pub trait ClusterStorage: Send + Sync {
    fn fetch_cluster(&self, cluster_id: u32) -> Option<ClusterData>;
}

/// In-memory cache for unit tests and instant mock data
pub struct MemoryClusterCache {
    clusters: std::collections::HashMap<u32, ClusterData>,
}

impl MemoryClusterCache {
    pub fn new() -> Self {
        Self {
            clusters: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: u32, data: ClusterData) {
        self.clusters.insert(id, data);
    }
}

impl ClusterStorage for MemoryClusterCache {
    fn fetch_cluster(&self, cluster_id: u32) -> Option<ClusterData> {
        self.clusters.get(&cluster_id).cloned()
    }
}

/// Persistent file-backed cache for disk streaming
pub struct FileClusterCache {
    base_path: std::path::PathBuf,
}

impl FileClusterCache {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

impl ClusterStorage for FileClusterCache {
    fn fetch_cluster(&self, cluster_id: u32) -> Option<ClusterData> {
        let file_path = self.base_path.join(format!("{}.cluster", cluster_id));
        
        let file_data = std::fs::read(&file_path).ok()?;
        
        // We assume the binary format matches the internal ClusterData layout.
        // For production, we'd need a proper deserialization/header check.
        // As a simple placeholder, we attempt to cast the first 40 bytes to a ClusterHeader.
        if file_data.len() < std::mem::size_of::<ClusterHeader>() {
            return None;
        }
        
        let (header_bytes, payload) = file_data.split_at(std::mem::size_of::<ClusterHeader>());
        let header: &ClusterHeader = bytemuck::try_from_bytes(header_bytes).ok()?;
        
        // Next extract vertices and indices
        let vertex_bytes_len = header.vertex_count as usize * 16; // arbitrary vertex stride assumption
        let index_bytes_len = header.index_count as usize * 4;
        
        if payload.len() < vertex_bytes_len + index_bytes_len {
            return None;
        }
        
        let vertex_data = payload[..vertex_bytes_len].to_vec();
        let index_data = payload[vertex_bytes_len..vertex_bytes_len + index_bytes_len].to_vec();

        Some(ClusterData {
            header: *header,
            vertex_data,
            index_data,
        })
    }
}

pub struct VirtualGeometrySystem {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl VirtualGeometrySystem {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Virtual Geometry LOD Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("cluster_lod.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Virtual Geometry Bind Group Layout"),
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Virtual Geometry Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Virtual Geometry Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Evaluates which clusters survive culling and selects the appropriate LOD.
    pub fn dispatch_cluster_evaluation(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera_buffer: &wgpu::Buffer,
        metadata_buffer: &wgpu::Buffer,
        output_buffer: &wgpu::Buffer,
        draw_count_buffer: &wgpu::Buffer,
        page_fault_buffer: &wgpu::Buffer,
        num_clusters: u32,
        device: &wgpu::Device,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Virtual Geometry Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: draw_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: page_fault_buffer.as_entire_binding(),
                },
            ],
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Virtual Geometry Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroups = (num_clusters + 63) / 64;
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }
}

pub struct ClusterStreamer {
    request_tx: tokio::sync::mpsc::Sender<u32>,
    response_rx: tokio::sync::mpsc::Receiver<ClusterData>,
}

impl ClusterStreamer {
    pub fn new(storage: std::sync::Arc<dyn ClusterStorage>) -> Self {
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel::<u32>(1024);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel::<ClusterData>(1024);

        let task = async move {
            while let Some(cluster_id) = request_rx.recv().await {
                if let Some(cluster_data) = storage.fetch_cluster(cluster_id) {
                    if response_tx.send(cluster_data).await.is_err() {
                        break;
                    }
                }
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(task);

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);

        Self {
            request_tx,
            response_rx,
        }
    }

    /// Request a cluster ID to be streamed into VRAM asynchronously.
    pub fn request_cluster(&self, cluster_id: u32) {
        // We use try_send to avoid stalling the main thread if the queue is full.
        let _ = self.request_tx.try_send(cluster_id);
    }

    /// Process the staging queue and upload newly streamed clusters to GPU VRAM.
    pub fn process_staging_queue(
        &mut self,
        queue: &wgpu::Queue,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
    ) {
        while let Ok(cluster_data) = self.response_rx.try_recv() {
            // Upload vertex data
            let vertex_offset = cluster_data.header.vertex_offset as wgpu::BufferAddress;
            queue.write_buffer(vertex_buffer, vertex_offset, &cluster_data.vertex_data);

            // Upload index data
            let index_offset = cluster_data.header.index_offset as wgpu::BufferAddress;
            queue.write_buffer(index_buffer, index_offset, &cluster_data.index_data);
        }
    }

    /// Readback page faults asynchronously without stalling the CPU (N-frame delayed)
    pub fn readback_page_faults(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        page_fault_buffer: &wgpu::Buffer,
        max_faults: u32,
    ) {
        let buffer_size = (4 + max_faults * 4) as wgpu::BufferAddress;
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Page Fault Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Page Fault Readback Encoder"),
        });

        encoder.copy_buffer_to_buffer(page_fault_buffer, 0, &staging_buffer, 0, buffer_size);

        // Reset the fault count to 0 for the next frame

        encoder.clear_buffer(page_fault_buffer, 0, Some(4));

        queue.submit(std::iter::once(encoder.finish()));

        let request_tx = self.request_tx.clone();

        // We move staging_buffer into a shared Arc to satisfy static lifetime requirements
        // of wgpu::Buffer::map_async.
        let staging_buffer = std::sync::Arc::new(staging_buffer);
        let buffer_slice_arc = staging_buffer.clone();

        staging_buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            if result.is_ok() {
                let data = buffer_slice_arc.slice(..).get_mapped_range();
                let mut offset = 0;
                let mut count_bytes = [0u8; 4];
                count_bytes.copy_from_slice(&data[0..4]);
                let fault_count = u32::from_ne_bytes(count_bytes);
                offset += 4;

                // Process up to max_faults or fault_count, whichever is smaller
                let limit = fault_count.min(max_faults);

                for _ in 0..limit {
                    let mut id_bytes = [0u8; 4];
                    id_bytes.copy_from_slice(&data[offset..offset + 4]);
                    let cluster_id = u32::from_ne_bytes(id_bytes);
                    let _ = request_tx.try_send(cluster_id);
                    offset += 4;
                }
            }
        });
    }
}
