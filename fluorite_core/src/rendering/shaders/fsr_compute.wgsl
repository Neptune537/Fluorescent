@group(0) @binding(0) var fsr_data: texture_2d<f32>; // Mock input for FSR
@group(0) @binding(1) var out_color: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dims = textureDimensions(out_color);

    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    // Placeholder FSR compute logic
    let color = textureLoad(fsr_data, coords, 0);
    textureStore(out_color, coords, color);
}
