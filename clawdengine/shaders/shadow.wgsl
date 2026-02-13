// Shadow depth-only pass — renders scene from light's perspective
struct LightVP {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> light_vp: LightVP;

struct ModelUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0) var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_shadow(in: VertexInput) -> @builtin(position) vec4<f32> {
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    return light_vp.view_proj * world_pos;
}
