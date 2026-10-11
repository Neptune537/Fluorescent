@group(0) @binding(0) var history_texture: texture_2d<f32>;
@group(0) @binding(1) var history_sampler: sampler;

@group(1) @binding(0) var velocity_texture: texture_2d<f32>;
@group(1) @binding(1) var velocity_sampler: sampler;

@group(2) @binding(0) var current_color_texture: texture_2d<f32>;
@group(2) @binding(1) var out_color: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dims = textureDimensions(out_color);

    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    let uv = vec2<f32>(coords) / vec2<f32>(dims);

    // Sample velocity to reproject history
    let velocity = textureSampleLevel(velocity_texture, velocity_sampler, uv, 0.0).xy;
    let history_uv = uv - velocity;

    // Sample history and current color
    let history_color = textureSampleLevel(history_texture, history_sampler, history_uv, 0.0);
    let current_color = textureLoad(current_color_texture, coords, 0);

    // Simple temporal blending (placeholder logic)
    let blend_factor = 0.9;
    let final_color = mix(current_color, history_color, blend_factor);

    textureStore(out_color, coords, final_color);
}
