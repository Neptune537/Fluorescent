use std::sync::Arc;

pub struct VirtualShadowMapConfig {
    pub resolution_per_page: u32,
    pub max_pages: u32,
    pub atlas_size_pages: u32,
}

impl Default for VirtualShadowMapConfig {
    fn default() -> Self {
        Self {
            resolution_per_page: 128,
            max_pages: 1024,
            atlas_size_pages: 8, // 8x8 = 64 physical pages
        }
    }
}

pub struct VirtualShadowMapSystem {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub config: VirtualShadowMapConfig,

    pub physical_atlas: wgpu::Texture,
    pub physical_atlas_view: wgpu::TextureView,
    pub page_table_buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
}

impl VirtualShadowMapSystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, config: VirtualShadowMapConfig) -> Self {
        let atlas_size = config.resolution_per_page * config.atlas_size_pages;
        let physical_atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VSM Physical Atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let physical_atlas_view = physical_atlas.create_view(&wgpu::TextureViewDescriptor::default());

        let page_table_size = (config.max_pages as wgpu::BufferAddress) * 4;
        let page_table_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VSM Page Table"),
            size: page_table_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VSM Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VSM Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&physical_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: page_table_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VSM Dummy Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@compute @workgroup_size(8, 8) fn main() {}",
            )),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VSM Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VSM Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Self {
            device,
            queue,
            config,
            physical_atlas,
            physical_atlas_view,
            page_table_buffer,
            bind_group_layout,
            bind_group,
            compute_pipeline,
        }
    }

    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, _light_dir: [f32; 3]) {
        // 1. Allocate physical pages in the `physical_atlas` based on the camera frustum and light direction.
        let mut allocated_pages = Vec::new();
        let num_physical_pages = self.config.atlas_size_pages * self.config.atlas_size_pages;
        let num_pages_to_allocate = self.config.max_pages.min(num_physical_pages);
        for i in 0..num_pages_to_allocate {
            // Compute page coordinates in the physical atlas (e.g., 8x8 grid)
            let page_x = i % self.config.atlas_size_pages;
            let page_y = i / self.config.atlas_size_pages;
            allocated_pages.push([page_x, page_y]);
        }

        // 2. Update the `page_table_buffer` to map virtual memory pages to physical atlas coordinates.
        let mut page_table_data = Vec::with_capacity(self.config.max_pages as usize);
        for i in 0..self.config.max_pages {
            if (i as usize) < allocated_pages.len() {
                let [px, py] = allocated_pages[i as usize];
                // Pack physical coordinates (x, y) into a u32 (16 bits each)
                let packed = (px & 0xFFFF) | ((py & 0xFFFF) << 16);
                page_table_data.push(packed); // mapped
            } else {
                page_table_data.push(u32::MAX); // unmapped
            }
        }

        // Write the updated page table to the buffer
        let data: &[u8] = bytemuck::cast_slice(&page_table_data);

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VSM Page Table Staging"),
            size: data.len() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: true,
        });
        staging_buffer.slice(..).get_mapped_range_mut().copy_from_slice(data);
        staging_buffer.unmap();

        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.page_table_buffer,
            0,
            data.len() as wgpu::BufferAddress,
        );

        // 3. Dispatch shadow rendering compute or render passes to write depth into the physical atlas.
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("VSM Shadow Rendering Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        // Bind the VSM specific bind group (including physical atlas and page table buffer)
        compute_pass.set_bind_group(0, &self.bind_group, &[]);

        // Dispatch compute shaders based on light frustums for shadow rendering
        let workgroup_size_x = self.config.resolution_per_page / 8;
        let workgroup_size_y = self.config.resolution_per_page / 8;

        for _i in 0..allocated_pages.len() {
            // Dispatch a compute workgroup for each allocated page to write into the physical atlas.
            compute_pass.dispatch_workgroups(workgroup_size_x, workgroup_size_y, 1);
        }
    }
}
