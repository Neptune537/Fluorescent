// ============================================================================
// Fluorite Engine — Phase 3: Virtual Shadow Maps (VSM) Depth Pass
// File: shadow_depth.wgsl
// ============================================================================

struct ShadowPassUniforms {
    light_view_proj: mat4x4<f32>,
    vsm_page_offset: vec2<f32>,
    vsm_page_scale: vec2<f32>,
};

@group(0) @binding(0) var<uniform> shadow_pass: ShadowPassUniforms;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model_matrix * vec4<f32>(in.position, 1.0);
    let proj = shadow_pass.light_view_proj * world_pos;

    // Scale and offset clip position to route rendering to the correct VSM physical page
    var adjusted_clip = proj;
    adjusted_clip.x = proj.x * shadow_pass.vsm_page_scale.x + shadow_pass.vsm_page_offset.x * proj.w;
    adjusted_clip.y = proj.y * shadow_pass.vsm_page_scale.y + shadow_pass.vsm_page_offset.y * proj.w;

    out.clip_pos = adjusted_clip;
    return out;
}
