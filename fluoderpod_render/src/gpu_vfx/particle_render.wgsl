struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>, // Instance position
    @location(1) color: vec4<f32>,    // Instance color
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Billboard vertices (triangle strip for a quad)
var<private> billboard_pos: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5, -0.5),
    vec2<f32>(-0.5,  0.5),
    vec2<f32>( 0.5,  0.5)
);

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // The particle position is the center. We add the billboard offset.
    // In a real system, we'd multiply the billboard offset by the right/up vectors
    // extracted from the view matrix to face the camera.

    // Simplistic billboard facing positive Z
    let offset = billboard_pos[input.vertex_index];
    let world_pos = input.position + vec3<f32>(offset.x, offset.y, 0.0);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = input.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple color output, could include texture sampling or soft particle depth testing
    return in.color;
}
