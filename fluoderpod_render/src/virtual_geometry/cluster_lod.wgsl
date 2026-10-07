struct CameraUniform {
    position: vec3<f32>,
    _padding: f32,
    view_proj: mat4x4<f32>,
    viewport_size: vec2<f32>,
    fov: f32,
    _padding2: f32,
    debug_page_faults: u32,
    _padding3: vec3<f32>,
    // 6 frustum planes (nx, ny, nz, d)
    frustum_planes: array<vec4<f32>, 6>,
}

struct ClusterHeader {
    sphere_bounds: vec4<f32>,
    lod_error: f32,
    parent_lod_error: f32,
    cluster_id: u32,
    parent_cluster_id: u32,
    vertex_offset: u32,
    vertex_count: u32,
    index_offset: u32,
    index_count: u32,
}

struct ClusterMetadataBuffer {
    clusters: array<ClusterHeader>,
}

struct DrawIndexedIndirectArgs {
    index_count: u32,
    instance_count: atomic<u32>,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

struct OutputDrawArgsBuffer {
    args: array<DrawIndexedIndirectArgs>,
}

struct PageFaultBuffer {
    count: atomic<u32>,
    cluster_ids: array<u32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> metadata_buffer: ClusterMetadataBuffer;
// Actually output indirect draw calls instead of a compact list of cluster ids
@group(0) @binding(2) var<storage, read_write> draw_args_buffer: OutputDrawArgsBuffer;
@group(0) @binding(3) var<storage, read_write> draw_count: atomic<u32>;
@group(0) @binding(4) var<storage, read_write> page_faults: PageFaultBuffer;

fn get_screen_space_error(cluster_error: f32, center: vec3<f32>, radius: f32) -> f32 {
    let dist = length(center - camera.position);
    let clamped_dist = max(dist - radius, 0.0001);

    // rho = (error / max(dist - r, epsilon)) * (height / (2 * tan(fov / 2)))
    let error_factor = cluster_error / clamped_dist;
    let projection_factor = camera.viewport_size.y / (2.0 * tan(camera.fov / 2.0));

    return error_factor * projection_factor;
}

fn is_sphere_in_frustum(center: vec3<f32>, radius: f32) -> bool {
    for (var i = 0u; i < 6u; i = i + 1u) {
        let plane = camera.frustum_planes[i];
        let distance = dot(plane.xyz, center) + plane.w;
        if distance < -radius {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Bounds check
    let num_clusters = arrayLength(&metadata_buffer.clusters);
    if idx >= num_clusters {
        return;
    }

    let cluster = metadata_buffer.clusters[idx];

    // Evaluate error
    let cluster_sse = get_screen_space_error(cluster.lod_error, cluster.sphere_bounds.xyz, cluster.sphere_bounds.w);
    let parent_sse = get_screen_space_error(cluster.parent_lod_error, cluster.sphere_bounds.xyz, cluster.sphere_bounds.w);

    let threshold = 1.0; // 1.0 pixel threshold

    // The root node has parent_lod_error = infinity (or a very large number), so it will always pass the parent condition.
    // If the cluster error is within the threshold, but the parent's error is too large, we select this cluster.
    if cluster_sse <= threshold && parent_sse > threshold {
        // Frustum cull
        if !is_sphere_in_frustum(cluster.sphere_bounds.xyz, cluster.sphere_bounds.w) {
            return;
        }

        // Check if the cluster is resident in memory (vertex_offset != 0xFFFFFFFF)
        if cluster.vertex_offset != 0xFFFFFFFFu {
            // Output an indirect draw call
            let output_idx = atomicAdd(&draw_count, 1u);
            draw_args_buffer.args[output_idx].index_count = cluster.index_count;
            atomicStore(&draw_args_buffer.args[output_idx].instance_count, 1u);
            draw_args_buffer.args[output_idx].first_index = cluster.index_offset;
            draw_args_buffer.args[output_idx].base_vertex = i32(cluster.vertex_offset);
            draw_args_buffer.args[output_idx].first_instance = cluster.cluster_id;
        } else {
            // Cluster is not resident, record a page fault
            let fault_idx = atomicAdd(&page_faults.count, 1u);
            // Limit page faults to avoid buffer overflow if there are too many
            let max_faults = arrayLength(&page_faults.cluster_ids);
            if fault_idx < max_faults {
                page_faults.cluster_ids[fault_idx] = cluster.cluster_id;
            }
        }
    }
}

