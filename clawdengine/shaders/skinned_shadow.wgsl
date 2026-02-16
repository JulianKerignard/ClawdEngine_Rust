// Shadow depth-only pass for skinned meshes
struct LightVP {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> light_vp: LightVP;

struct ModelUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0) var<uniform> model: ModelUniforms;

// Group 2: Joint matrices
struct JointMatrices {
    matrices: array<mat4x4<f32>, 128>,
};
@group(2) @binding(0) var<uniform> joints: JointMatrices;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
};

@vertex
fn vs_shadow(in: VertexInput) -> @builtin(position) vec4<f32> {
    let w = in.joint_weights;
    let skin_matrix =
        joints.matrices[in.joint_indices.x] * w.x +
        joints.matrices[in.joint_indices.y] * w.y +
        joints.matrices[in.joint_indices.z] * w.z +
        joints.matrices[in.joint_indices.w] * w.w;
    let skinned_pos = skin_matrix * vec4<f32>(in.position, 1.0);
    let world_pos = model.model * skinned_pos;
    return light_vp.view_proj * world_pos;
}
