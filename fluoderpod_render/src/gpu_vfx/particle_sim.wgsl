struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec4<f32>,
    age: f32,
    lifetime: f32,
}

struct ParticleConfig {
    delta_time: f32,
    _pad0: vec3<f32>,
    gravity: vec3<f32>,
    _pad1: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> config: ParticleConfig;
@group(0) @binding(2) var depth_texture: texture_depth_2d;
@group(0) @binding(3) var depth_sampler: sampler;
@group(0) @binding(4) var<uniform> view_proj: mat4x4<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    var particle = particles[index];

    if (particle.age >= particle.lifetime) {
        // Reset or kill particle (handling simplified for this example)
        return;
    }

    // Update velocity with gravity
    particle.velocity += config.gravity * config.delta_time;

    // Update position
    var next_position = particle.position + particle.velocity * config.delta_time;

    // --- Depth Collision (Simplified Screen Space) ---
    // Project next position to clip space
    let clip_pos = view_proj * vec4<f32>(next_position, 1.0);
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC to UV (0.0 to 1.0)
    let uv = vec2<f32>(ndc_pos.x * 0.5 + 0.5, -ndc_pos.y * 0.5 + 0.5);

    if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
        // Load depth directly (no filtering)
        let dims = textureDimensions(depth_texture);
        let coord = vec2<i32>(uv * vec2<f32>(dims));
        let scene_depth = textureLoad(depth_texture, coord, 0);

        // Very basic collision: if particle is behind the depth buffer
        if (ndc_pos.z > scene_depth) {
            // Bounce
            particle.velocity = reflect(particle.velocity, vec3<f32>(0.0, 1.0, 0.0)) * 0.5; // Assuming flat ground for simplicity
            next_position = particle.position; // Revert position
        }
    }

    particle.position = next_position;
    particle.age += config.delta_time;

    // Write back
    particles[index] = particle;
}
