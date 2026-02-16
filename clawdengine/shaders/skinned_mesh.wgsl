// Skinned mesh shader — extends mesh.wgsl with vertex skinning via joint matrices

// ---- Group 0: Camera ----
struct CameraUniforms {
    view_proj: mat4x4<f32>,
    eye_position: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniforms;

// ---- Group 1: Per-object model ----
struct ModelUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0) var<uniform> model: ModelUniforms;

// ---- Group 2: Material ----
struct MaterialUniforms {
    albedo: vec4<f32>,
    roughness: f32,
    metallic: f32,
    emission: vec4<f32>,
};
@group(2) @binding(0) var<uniform> material: MaterialUniforms;
@group(2) @binding(1) var t_albedo: texture_2d<f32>;
@group(2) @binding(2) var s_albedo: sampler;
@group(2) @binding(3) var t_normal: texture_2d<f32>;

// ---- Group 3: Lights ----
struct LightData {
    position: vec4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
    spot_params: vec4<f32>,
};
struct LightsUniforms {
    ambient: vec4<f32>,
    count: u32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    lights: array<LightData, 4>,
};
@group(3) @binding(0) var<uniform> lights: LightsUniforms;

// ---- Group 4: Shadow ----
struct LightVP {
    view_proj: mat4x4<f32>,
};
@group(4) @binding(0) var t_shadow: texture_depth_2d;
@group(4) @binding(1) var s_shadow: sampler_comparison;
@group(4) @binding(2) var<uniform> light_vp: LightVP;

// ---- Group 5: Joint Matrices ----
struct JointMatrices {
    matrices: array<mat4x4<f32>, 128>,
};
@group(5) @binding(0) var<uniform> joints: JointMatrices;

// ---- Skinned Vertex ----
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) shadow_pos: vec4<f32>,
    @location(4) world_tangent: vec3<f32>,
    @location(5) world_bitangent: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // Linear Blend Skinning
    let w = in.joint_weights;
    let skin_matrix =
        joints.matrices[in.joint_indices.x] * w.x +
        joints.matrices[in.joint_indices.y] * w.y +
        joints.matrices[in.joint_indices.z] * w.z +
        joints.matrices[in.joint_indices.w] * w.w;

    let skinned_pos = skin_matrix * vec4<f32>(in.position, 1.0);
    let skinned_normal = (skin_matrix * vec4<f32>(in.normal, 0.0)).xyz;
    let skinned_tangent_xyz = (skin_matrix * vec4<f32>(in.tangent.xyz, 0.0)).xyz;

    // Apply model matrix (identical to mesh.wgsl from here)
    var out: VertexOutput;
    let world_pos = model.model * skinned_pos;
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = (model.model * vec4<f32>(skinned_normal, 0.0)).xyz;
    out.world_position = world_pos.xyz;
    out.uv = in.uv;
    out.shadow_pos = light_vp.view_proj * world_pos;

    // TBN frame
    let raw_T = (model.model * vec4<f32>(skinned_tangent_xyz, 0.0)).xyz;
    let N_raw = normalize((model.model * vec4<f32>(skinned_normal, 0.0)).xyz);
    let tan_len = length(raw_T);
    if (tan_len > 0.0001) {
        let T = raw_T / tan_len;
        out.world_tangent = T;
        out.world_bitangent = cross(N_raw, T) * in.tangent.w;
    } else {
        out.world_tangent = vec3<f32>(0.0, 0.0, 0.0);
        out.world_bitangent = vec3<f32>(0.0, 0.0, 0.0);
    }
    return out;
}

// ---- Shadow PCF (identical to mesh.wgsl) ----
fn calc_shadow(shadow_pos: vec4<f32>) -> f32 {
    let proj = shadow_pos.xyz / shadow_pos.w;
    let uv = proj.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || proj.z > 1.0) {
        return 1.0;
    }
    let texel_size = 1.0 / 2048.0;
    var shadow = 0.0;
    for (var x = -2i; x <= 1i; x++) {
        for (var y = -2i; y <= 1i; y++) {
            let offset = (vec2<f32>(f32(x), f32(y)) + 0.5) * texel_size;
            shadow += textureSampleCompare(t_shadow, s_shadow, uv + offset, proj.z);
        }
    }
    return shadow / 16.0;
}

// ---- Fragment: Blinn-Phong (identical to mesh.wgsl) ----
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N_geom = normalize(in.world_normal);
    var N = N_geom;
    let tan_len = length(in.world_tangent);
    if (tan_len > 0.0001) {
        let T = normalize(in.world_tangent);
        let B = normalize(in.world_bitangent);
        let normal_sample = textureSample(t_normal, s_albedo, in.uv).rgb;
        let tangent_normal = normal_sample * 2.0 - vec3<f32>(1.0, 1.0, 1.0);
        let TBN = mat3x3<f32>(T, B, N_geom);
        N = normalize(TBN * tangent_normal);
    }
    let V = normalize(camera.eye_position.xyz - in.world_position);
    let tex_color = textureSample(t_albedo, s_albedo, in.uv);
    let base_color = material.albedo.rgb * tex_color.rgb;

    var result = base_color * lights.ambient.rgb;

    for (var i = 0u; i < min(lights.count, 4u); i++) {
        let light = lights.lights[i];
        let intensity = light.color.w;

        var L: vec3<f32>;
        var attenuation = 1.0;
        if (light.position.w < 0.5) {
            L = normalize(-light.direction.xyz);
        } else {
            let to_light = light.position.xyz - in.world_position;
            let dist = length(to_light);
            L = to_light / max(dist, 0.001);
            let range = light.direction.w;
            if (range > 0.0) {
                let dist_ratio = clamp(dist / range, 0.0, 1.0);
                let falloff = pow(1.0 - pow(dist_ratio, 4.0), 2.0);
                attenuation = falloff / (dist * dist + 1.0);
            } else {
                attenuation = 1.0 / (1.0 + 0.09 * dist + 0.032 * dist * dist);
            }

            if (light.position.w > 1.5) {
                let spot_dir = normalize(light.direction.xyz);
                let cos_inner = light.spot_params.x;
                let cos_outer = light.spot_params.y;
                let theta = dot(-L, spot_dir);
                let spot_factor = smoothstep(cos_outer, cos_inner, theta);
                attenuation *= spot_factor;
            }
        }

        var shadow = 1.0;
        if (light.position.w < 0.5 && i == 0u) {
            shadow = calc_shadow(in.shadow_pos);
        }

        let NdotL = max(dot(N, L), 0.0);
        let diffuse = light.color.rgb * intensity * NdotL * attenuation * shadow;

        let H = normalize(L + V);
        let shininess = mix(16.0, 128.0, 1.0 - material.roughness);
        let NdotH = max(dot(N, H), 0.0);
        let spec_strength = pow(NdotH, shininess);
        let fresnel = mix(vec3<f32>(0.04, 0.04, 0.04), base_color, material.metallic);
        let specular = light.color.rgb * intensity * spec_strength * attenuation * shadow * fresnel;

        result += base_color * diffuse + specular;
    }

    result += material.emission.rgb;

    if (model.color.a > 0.5) {
        result = mix(result, model.color.rgb, 0.3);
    }

    return vec4<f32>(result, material.albedo.a);
}
