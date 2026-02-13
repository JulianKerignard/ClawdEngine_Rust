// Skybox gradient shader — fullscreen triangle, no vertex buffer
// Uses same CameraUniforms as mesh.wgsl (group 0)

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    eye_position: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) clip_y: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle: 3 vertices covering entire clip space
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );

    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.clip_position = vec4<f32>(pos.x, pos.y, 0.9999, 1.0);
    out.clip_y = pos.y;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize Y from clip space [-1, 1] to [0, 1]
    let t = in.clip_y * 0.5 + 0.5; // 0 = bottom, 1 = top

    // Color stops
    let col_top     = vec3<f32>(0.05, 0.08, 0.18); // deep blue
    let col_mid     = vec3<f32>(0.15, 0.22, 0.42); // medium blue
    let col_horizon = vec3<f32>(0.35, 0.40, 0.55); // light blue
    let col_bottom  = vec3<f32>(0.08, 0.08, 0.10); // dark grey

    // Gradient: bottom → horizon → mid → top
    var color: vec3<f32>;
    if (t < 0.5) {
        // Bottom half: ground grey → horizon
        let s = t * 2.0; // 0..1
        color = mix(col_bottom, col_horizon, smoothstep(0.0, 1.0, s));
    } else {
        // Top half: horizon → mid → top
        let s = (t - 0.5) * 2.0; // 0..1
        let mid_to_top = mix(col_mid, col_top, smoothstep(0.0, 1.0, s));
        color = mix(col_horizon, mid_to_top, smoothstep(0.0, 1.0, s));
    }

    return vec4<f32>(color, 1.0);
}
