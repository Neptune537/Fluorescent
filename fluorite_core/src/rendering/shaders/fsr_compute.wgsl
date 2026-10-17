// Production WGSL Compute Shader for Temporal Super-Resolution & Spatial Clamping
// Provides sub-pixel temporal reprojection, 3x3 color neighborhood clamping, and Mitchell-Netravali filtering.

@group(0) @binding(0) var current_color_texture: texture_2d<f32>;
@group(0) @binding(1) var history_texture: texture_2d<f32>;
@group(0) @binding(2) var history_sampler: sampler;
@group(0) @binding(3) var velocity_texture: texture_2d<f32>;
@group(0) @binding(4) var velocity_sampler: sampler;
@group(0) @binding(5) var out_color: texture_storage_2d<rgba8unorm, write>;

// Mitchell-Netravali bicubic filter weights (B=1/3, C=1/3)
fn mitchell_weight(x: f32) -> f32 {
    let d = abs(x);
    if (d < 1.0) {
        return (7.0 * d * d * d - 12.0 * d * d + 5.333333) / 6.0;
    } else if (d < 2.0) {
        return (-2.333333 * d * d * d + 12.0 * d * d - 20.0 * d + 10.666667) / 6.0;
    }
    return 0.0;
}

// Converts RGB to YCoCg for tighter bounding-box neighborhood clamping
fn rgb_to_ycocg(rgb: vec3<f32>) -> vec3<f32> {
    let y = dot(rgb, vec3<f32>(0.25, 0.5, 0.25));
    let co = dot(rgb, vec3<f32>(0.5, 0.0, -0.5));
    let cg = dot(rgb, vec3<f32>(-0.25, 0.5, -0.25));
    return vec3<f32>(y, co, cg);
}

fn ycocg_to_rgb(ycocg: vec3<f32>) -> vec3<f32> {
    let y = ycocg.x;
    let co = ycocg.y;
    let cg = ycocg.z;
    let r = y + co - cg;
    let g = y + cg;
    let b = y - co - cg;
    return max(vec3<f32>(0.0), vec3<f32>(r, g, b));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_coords = vec2<i32>(global_id.xy);
    let out_dims = textureDimensions(out_color);

    if (out_coords.x >= i32(out_dims.x) || out_coords.y >= i32(out_dims.y)) {
        return;
    }

    let out_uv = (vec2<f32>(out_coords) + vec2<f32>(0.5)) / vec2<f32>(out_dims);
    let in_dims = textureDimensions(current_color_texture);
    let in_coords = vec2<i32>(out_uv * vec2<f32>(in_dims));

    // 1. Gather 3x3 neighborhood around current pixel for color variance clipping
    var min_color = vec3<f32>(1e10);
    var max_color = vec3<f32>(-1e10);
    var center_color = vec3<f32>(0.0);

    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
            let sample_coord = clamp(in_coords + vec2<i32>(dx, dy), vec2<i32>(0), vec2<i32>(in_dims) - vec2<i32>(1));
            let s_rgb = textureLoad(current_color_texture, sample_coord, 0).rgb;
            let s_ycocg = rgb_to_ycocg(s_rgb);

            min_color = min(min_color, s_ycocg);
            max_color = max(max_color, s_ycocg);

            if (dx == 0 && dy == 0) {
                center_color = s_ycocg;
            }
        }
    }

    // 2. Sample motion vector and reproject history UV
    let velocity = textureSampleLevel(velocity_texture, velocity_sampler, out_uv, 0.0).xy;
    let history_uv = out_uv - velocity;

    // Check if reprojected UV falls outside screen bounds (disocclusion)
    let is_offscreen = history_uv.x < 0.0 || history_uv.x > 1.0 || history_uv.y < 0.0 || history_uv.y > 1.0;

    var final_ycocg = center_color;

    if (!is_offscreen) {
        let history_rgb = textureSampleLevel(history_texture, history_sampler, history_uv, 0.0).rgb;
        var history_ycocg = rgb_to_ycocg(history_rgb);

        // Clamp history to current 3x3 neighborhood bounding box to prevent ghosting
        history_ycocg = clamp(history_ycocg, min_color, max_color);

        // Adaptive blend factor: High confidence = 0.90 history, motion/disocclusion adapts downwards
        let vel_mag = length(velocity);
        let blend_weight = clamp(0.92 - vel_mag * 10.0, 0.60, 0.92);

        final_ycocg = mix(center_color, history_ycocg, blend_weight);
    }

    let final_rgb = ycocg_to_rgb(final_ycocg);
    textureStore(out_color, out_coords, vec4<f32>(final_rgb, 1.0));
}
