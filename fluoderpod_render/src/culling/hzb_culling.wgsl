struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0: f32,
    frustum_planes: array<vec4<f32>, 6>,
    viewport_size: vec2<f32>,
    _pad1: vec2<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

struct PackedEntityInstance {
    pos_and_radius: vec4<f32>,     // xyz = position / sphere_center, w = sphere_radius
    rotation_quat: vec4<f32>,      // xyzw = unit quaternion
    scale_and_meta: vec4<f32>,     // xyz = scale, w = bitcast<f32>(meta_or_color)
};

@group(0) @binding(1) var<storage, read> instances: array<PackedEntityInstance>;

struct DrawIndexedIndirectArgs {
    index_count: u32,
    instance_count: atomic<u32>,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

@group(0) @binding(2) var<storage, read_write> draw_args: DrawIndexedIndirectArgs;
@group(0) @binding(3) var<storage, read_write> compact_instances: array<u32>;
@group(0) @binding(4) var hzb_tex: texture_2d<f32>;
@group(0) @binding(5) var hzb_sampler: sampler;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let instance_index = global_id.x;

    if (instance_index >= arrayLength(&instances)) {
        return;
    }

    let instance = instances[instance_index];
    let center = instance.pos_and_radius.xyz;
    let radius = instance.pos_and_radius.w;

    // 1. Frustum Culling
    var is_visible = true;
    for (var i = 0u; i < 6u; i = i + 1u) {
        let plane = camera.frustum_planes[i];
        if (dot(plane.xyz, center) + plane.w < -radius) {
            is_visible = false;
            break;
        }
    }

    // 2. HZB Occlusion Culling
    if (is_visible) {
        // Dynamically compute conservative AABB
        let aabb_min = center - vec3<f32>(radius);
        let aabb_max = center + vec3<f32>(radius);

        // Project AABB corners to NDC
        var min_xy = vec2<f32>(1.0, 1.0);
        var max_xy = vec2<f32>(-1.0, -1.0);
        var max_z = -1.0; // Reverse-Z: max_z is nearest. Init with far value.

        let corners = array<vec3<f32>, 8>(
            vec3<f32>(aabb_min.x, aabb_min.y, aabb_min.z),
            vec3<f32>(aabb_max.x, aabb_min.y, aabb_min.z),
            vec3<f32>(aabb_min.x, aabb_max.y, aabb_min.z),
            vec3<f32>(aabb_max.x, aabb_max.y, aabb_min.z),
            vec3<f32>(aabb_min.x, aabb_min.y, aabb_max.z),
            vec3<f32>(aabb_max.x, aabb_min.y, aabb_max.z),
            vec3<f32>(aabb_min.x, aabb_max.y, aabb_max.z),
            vec3<f32>(aabb_max.x, aabb_max.y, aabb_max.z)
        );

        var any_behind_near_plane = false;

        for (var i = 0u; i < 8u; i = i + 1u) {
            let clip = camera.view_proj * vec4<f32>(corners[i], 1.0);
            if (clip.w <= 0.0 || clip.z > clip.w) { // Behind near plane
                any_behind_near_plane = true;
            } else {
                let ndc = clip.xyz / clip.w;
                min_xy = min(min_xy, ndc.xy);
                max_xy = max(max_xy, ndc.xy);
                max_z = max(max_z, ndc.z); // Track the maximum Z value (nearest point in Reverse-Z)
            }
        }

        if (!any_behind_near_plane) {
            // Convert NDC to UV [0, 1]
            let uv_min = min_xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
            let uv_max = max_xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

            let rect_min = min(uv_min, uv_max);
            let rect_max = max(uv_min, uv_max);

            let screen_size = (rect_max - rect_min) * camera.viewport_size;
            let max_dim = max(screen_size.x, screen_size.y);

            // Log2 of dimension to select mip level
            var mip = 0.0;
            if (max_dim > 1.0) {
                mip = ceil(log2(max_dim));
            }

            // 4-tap sampling of the HZB texture
            let d0 = textureSampleLevel(hzb_tex, hzb_sampler, vec2<f32>(rect_min.x, rect_min.y), mip).r;
            let d1 = textureSampleLevel(hzb_tex, hzb_sampler, vec2<f32>(rect_max.x, rect_min.y), mip).r;
            let d2 = textureSampleLevel(hzb_tex, hzb_sampler, vec2<f32>(rect_min.x, rect_max.y), mip).r;
            let d3 = textureSampleLevel(hzb_tex, hzb_sampler, vec2<f32>(rect_max.x, rect_max.y), mip).r;

            // In Reverse-Z, HZB stores furthest depth per tile (minimum value)
            // So we need the furthest of the 4 samples, which is the minimum of them.
            let hzb_depth = min(min(d0, d1), min(d2, d3));

            // object_nearest_depth < hzb_furthest_depth implies occluded in Reverse-Z
            if (max_z < hzb_depth) {
                is_visible = false;
            }
        }
    }

    if (is_visible) {
        let slot = atomicAdd(&draw_args.instance_count, 1u);
        compact_instances[slot] = instance_index;
    }
}
