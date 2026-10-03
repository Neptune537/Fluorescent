@group(0) @binding(0) var scene_color: texture_2d<f32>;
@group(0) @binding(1) var scene_depth: texture_depth_2d;
@group(0) @binding(2) var output_gi: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(scene_color);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    // Simplistic placeholder for Screen-Space Raymarching SSGI
    // In a full implementation, we'd reconstruct position and normal from depth/gbuffer
    // and raymarch against the depth buffer to accumulate bounce lighting.

    let base_color = textureLoad(scene_color, global_id.xy, 0);

    // Output placeholder GI contribution
    let gi_color = vec4<f32>(base_color.rgb * 0.1, 1.0);

    textureStore(output_gi, global_id.xy, gi_color);
}
