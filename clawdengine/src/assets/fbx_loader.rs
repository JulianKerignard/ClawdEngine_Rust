use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

use crate::assets::gltf_loader::{
    topological_sort_bones, GltfLoadedMaterial, GltfLoadedMesh, GltfLoadedSkinnedMesh,
    GltfLoadedTexture, GltfScene,
};
use crate::core::{
    AnimationChannel, AnimationClip, AnimationProperty, Bone, InterpolationMode, Skeleton,
    Transform, MAX_JOINTS,
};
use crate::renderer::mesh::{compute_tangents, Vertex};
use crate::renderer::skinned_mesh::SkinnedVertex;

// ---- Internal types ----

struct SkinData {
    skeleton: Skeleton,
    node_to_bone: HashMap<u32, usize>,
    cluster_to_bone: HashMap<usize, usize>,
}

// ---- Conversion helpers ----

fn v3(v: &ufbx::Vec3) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}

fn q4(q: &ufbx::Quat) -> Quat {
    Quat::from_xyzw(q.x as f32, q.y as f32, q.z as f32, q.w as f32)
}

fn m4(m: &ufbx::Matrix) -> Mat4 {
    Mat4::from_cols_array(&[
        m.m00 as f32, m.m10 as f32, m.m20 as f32, 0.0,
        m.m01 as f32, m.m11 as f32, m.m21 as f32, 0.0,
        m.m02 as f32, m.m12 as f32, m.m22 as f32, 0.0,
        m.m03 as f32, m.m13 as f32, m.m23 as f32, 1.0,
    ])
}

fn elem_name(el: &ufbx::Element) -> String {
    let n: &str = &el.name;
    if n.is_empty() { format!("node_{}", el.typed_id) } else { n.to_owned() }
}

// ---- Public API ----

pub fn load_fbx(path: impl AsRef<Path>) -> Result<GltfScene> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy().to_string();

    log::info!("[FBX] Loading file: {}", path_str);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    log::debug!("[FBX] File size: {} bytes ({:.1} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));

    let opts = ufbx::LoadOpts {
        target_axes: ufbx::CoordinateAxes {
            right: ufbx::CoordinateAxis::PositiveX,
            up: ufbx::CoordinateAxis::PositiveY,
            front: ufbx::CoordinateAxis::PositiveZ,
        },
        space_conversion: ufbx::SpaceConversion::AdjustTransforms,
        generate_missing_normals: true,
        clean_skin_weights: true,
        ..Default::default()
    };

    let parse_start = std::time::Instant::now();
    let fbx = ufbx::load_file(&path_str, opts)
        .map_err(|e| anyhow::anyhow!("FBX load failed: {:?}", e))?;
    log::debug!("[FBX] ufbx parse completed in {:.1}ms", parse_start.elapsed().as_secs_f64() * 1000.0);
    let sc: &ufbx::Scene = &fbx;

    log::debug!("[FBX] Scene stats: {} nodes, {} meshes, {} skin_deformers, {} anim_stacks, {} textures",
        sc.nodes.len(), sc.meshes.len(), sc.skin_deformers.len(), sc.anim_stacks.len(), sc.textures.len());

    let textures = extract_textures(sc, path);
    log::debug!("[FBX] Extracted {} textures", textures.len());
    let tex_map: HashMap<u32, usize> = (0..sc.textures.len().min(textures.len()))
        .map(|i| (sc.textures[i].element.typed_id, i))
        .collect();

    let mut skin_data: Vec<SkinData> = Vec::new();
    let mut skin_map: HashMap<u32, usize> = HashMap::new();
    for i in 0..sc.skin_deformers.len() {
        let sd = &sc.skin_deformers[i];
        log::debug!("[FBX] Processing skin deformer {} (id={}, clusters={})",
            i, sd.element.typed_id, sd.clusters.len());
        skin_map.insert(sd.element.typed_id, skin_data.len());
        skin_data.push(extract_skeleton(sd));
    }
    if skin_data.is_empty() {
        log::warn!("[FBX] No skin deformers found — this file has no skeletal data");
    }

    let mut result = GltfScene {
        meshes: Vec::new(),
        textures,
        skeletons: Vec::new(),
        skinned_meshes: Vec::new(),
        animation_clips: Vec::new(),
    };

    let mut skipped_nodes = 0u32;
    let mut mesh_nodes = 0u32;
    for i in 0..sc.nodes.len() {
        let node = &sc.nodes[i];
        if node.is_root || node.is_geometry_transform_helper || node.is_scale_helper {
            skipped_nodes += 1;
            continue;
        }
        if let Some(ref mesh_ref) = node.mesh {
            let node_name: &str = &node.element.name;
            log::debug!("[FBX] Processing mesh node '{}' (id={}, skin_deformers={})",
                node_name, node.element.typed_id, mesh_ref.skin_deformers.len());
            mesh_nodes += 1;
            process_node_mesh(
                node, mesh_ref, &skin_data, &skin_map, &tex_map, &path_str, &mut result,
            );
        }
    }
    log::debug!("[FBX] Node scan: {} total, {} skipped (root/helper), {} with meshes",
        sc.nodes.len(), skipped_nodes, mesh_nodes);

    result.animation_clips = extract_animations(sc, &skin_data);
    for sd in skin_data {
        result.skeletons.push(sd.skeleton);
    }

    log::info!(
        "Loaded FBX: {} ({} meshes, {} skinned, {} skels, {} anims, {} tex)",
        path_str, result.meshes.len(), result.skinned_meshes.len(),
        result.skeletons.len(), result.animation_clips.len(), result.textures.len(),
    );
    Ok(result)
}

// ---- Skeleton extraction ----

fn extract_skeleton(skin: &ufbx::SkinDeformer) -> SkinData {
    let cc = skin.clusters.len();
    let skin_name: &str = &skin.element.name;
    log::debug!("[FBX:Skeleton] Extracting skeleton '{}' with {} clusters", skin_name, cc);
    if cc == 0 {
        log::warn!("[FBX:Skeleton] Skeleton '{}' has 0 clusters — returning empty skeleton", skin_name);
        return SkinData {
            skeleton: Skeleton::new("empty".into(), vec![], 0),
            node_to_bone: HashMap::new(),
            cluster_to_bone: HashMap::new(),
        };
    }

    // First pass: collect bone node IDs → cluster index
    let mut node_to_cluster: HashMap<u32, usize> = HashMap::new();
    for i in 0..cc {
        if let Some(ref bn) = skin.clusters[i].bone_node {
            node_to_cluster.insert(bn.element.typed_id, i);
        }
    }

    // Second pass: build unsorted bones
    let mut unsorted: Vec<(usize, Bone)> = Vec::with_capacity(cc);
    for i in 0..cc {
        let cluster = &skin.clusters[i];
        let bn = match &cluster.bone_node {
            Some(n) => n,
            None => continue,
        };
        let parent_idx = bn
            .parent
            .as_ref()
            .and_then(|p| node_to_cluster.get(&p.element.typed_id).copied());

        let lt = &bn.local_transform;
        unsorted.push((
            i,
            Bone {
                name: elem_name(&bn.element),
                parent: parent_idx,
                children: Vec::new(),
                inverse_bind_matrix: m4(&cluster.geometry_to_bone),
                local_bind_transform: Transform {
                    position: v3(&lt.translation),
                    rotation: q4(&lt.rotation),
                    scale: v3(&lt.scale),
                },
            },
        ));
    }

    if unsorted.len() > MAX_JOINTS {
        log::warn!(
            "[FBX:Skeleton] {} bones exceeds limit {}, truncating",
            unsorted.len(),
            MAX_JOINTS
        );
        unsorted.truncate(MAX_JOINTS);
    }

    let bone_count_before_sort = unsorted.len();
    let (sorted, old_to_new) = topological_sort_bones(unsorted);
    let node_to_bone: HashMap<u32, usize> = node_to_cluster
        .iter()
        .filter_map(|(&nid, &ci)| old_to_new.get(&ci).map(|&bi| (nid, bi)))
        .collect();
    let root = sorted.iter().position(|b| b.parent.is_none()).unwrap_or(0);
    let name = {
        let n: &str = &skin.element.name;
        if n.is_empty() { "skeleton".to_string() } else { n.to_owned() }
    };

    log::info!("[FBX:Skeleton] '{}': {} bones (root='{}' at idx {}), {} node-to-bone mappings",
        name, sorted.len(), sorted.get(root).map(|b| b.name.as_str()).unwrap_or("?"), root, node_to_bone.len());
    if sorted.len() != bone_count_before_sort {
        log::warn!("[FBX:Skeleton] Bone count changed after topological sort: {} -> {} (orphaned bones?)",
            bone_count_before_sort, sorted.len());
    }
    // Log first few bone names for quick debugging
    let bone_names: Vec<&str> = sorted.iter().take(5).map(|b| b.name.as_str()).collect();
    log::debug!("[FBX:Skeleton] First bones: {:?}{}", bone_names,
        if sorted.len() > 5 { format!(" ... +{} more", sorted.len() - 5) } else { String::new() });

    SkinData {
        skeleton: Skeleton::new(name, sorted, root),
        node_to_bone,
        cluster_to_bone: old_to_new,
    }
}

// ---- Mesh extraction ----

fn process_node_mesh(
    node: &ufbx::Node,
    mesh: &ufbx::Mesh,
    skin_data: &[SkinData],
    skin_map: &HashMap<u32, usize>,
    tex_map: &HashMap<u32, usize>,
    file_path: &str,
    result: &mut GltfScene,
) {
    let name = elem_name(&node.element);
    // FBX is typically in cm — apply 0.01 scale to convert to engine units (meters)
    let cm_to_m = Mat4::from_scale(Vec3::splat(0.01));
    let transform = cm_to_m * m4(&node.node_to_world);
    let material = extract_material(mesh, tex_map);
    let store_name = format!("fbx:{}#{}", file_path, name);

    let skin_info: Option<(usize, &ufbx::SkinDeformer)> = if !mesh.skin_deformers.is_empty() {
        let sd = &mesh.skin_deformers[0];
        skin_map.get(&sd.element.typed_id).map(|&idx| (idx, sd))
    } else {
        None
    };

    if let Some((skin_idx, skin_deformer)) = skin_info {
        log::debug!("[FBX:Mesh] '{}' → skinned path (skin_idx={}, clusters={})",
            name, skin_idx, skin_deformer.clusters.len());
        let (verts, idxs) = build_skinned_verts(mesh, skin_deformer, &skin_data[skin_idx]);
        if !verts.is_empty() {
            log::debug!("[FBX:Mesh] '{}': {} skinned vertices, {} indices, {} triangles",
                name, verts.len(), idxs.len(), idxs.len() / 3);
            result.skinned_meshes.push(GltfLoadedSkinnedMesh {
                store_name,
                display_name: name,
                vertices: verts,
                indices: idxs,
                transform,
                material,
                skeleton_index: skin_idx,
            });
        } else {
            log::warn!("[FBX:Mesh] '{}': skinned vertex extraction produced 0 vertices", name);
        }
    } else {
        log::debug!("[FBX:Mesh] '{}' → static mesh path (no skin deformer)", name);
        let (mut verts, idxs) = build_static_verts(mesh);
        if !verts.is_empty() {
            log::debug!("[FBX:Mesh] '{}': {} static vertices, {} indices, {} triangles",
                name, verts.len(), idxs.len(), idxs.len() / 3);
            if !mesh.vertex_tangent.exists {
                compute_tangents(&mut verts, &idxs);
            }
            result.meshes.push(GltfLoadedMesh {
                store_name,
                display_name: name,
                vertices: verts,
                indices: idxs,
                transform,
                material,
            });
        } else {
            log::warn!("[FBX:Mesh] '{}': static vertex extraction produced 0 vertices", name);
        }
    }
}

fn build_static_verts(mesh: &ufbx::Mesh) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::with_capacity(mesh.num_triangles * 3);
    let mut idxs = Vec::with_capacity(mesh.num_triangles * 3);
    let (hn, hu, ht) = (mesh.vertex_normal.exists, mesh.vertex_uv.exists, mesh.vertex_tangent.exists);

    for fi in 0..mesh.num_faces {
        let face = &mesh.faces[fi];
        let begin = face.index_begin as usize;
        let count = face.num_indices as usize;
        if count < 3 { continue; }

        for tri in 0..(count - 2) {
            for &ci in &[begin, begin + tri + 1, begin + tri + 2] {
                let p = &mesh.vertex_position[ci];
                let idx = verts.len() as u32;
                verts.push(Vertex {
                    position: [p.x as f32, p.y as f32, p.z as f32],
                    normal: if hn { let n = &mesh.vertex_normal[ci]; [n.x as f32, n.y as f32, n.z as f32] } else { [0.0, 1.0, 0.0] },
                    uv: if hu { let u = &mesh.vertex_uv[ci]; [u.x as f32, u.y as f32] } else { [0.0, 0.0] },
                    tangent: if ht { let t = &mesh.vertex_tangent[ci]; [t.x as f32, t.y as f32, t.z as f32, 1.0] } else { [0.0, 0.0, 0.0, 1.0] },
                });
                idxs.push(idx);
            }
        }
    }
    (verts, idxs)
}

fn build_skinned_verts(
    mesh: &ufbx::Mesh,
    skin: &ufbx::SkinDeformer,
    sd: &SkinData,
) -> (Vec<SkinnedVertex>, Vec<u32>) {
    let mut verts = Vec::with_capacity(mesh.num_triangles * 3);
    let mut idxs = Vec::with_capacity(mesh.num_triangles * 3);
    let (hn, hu, ht) = (mesh.vertex_normal.exists, mesh.vertex_uv.exists, mesh.vertex_tangent.exists);

    for fi in 0..mesh.num_faces {
        let face = &mesh.faces[fi];
        let begin = face.index_begin as usize;
        let count = face.num_indices as usize;
        if count < 3 { continue; }

        for tri in 0..(count - 2) {
            for &ci in &[begin, begin + tri + 1, begin + tri + 2] {
                let p = &mesh.vertex_position[ci];
                let vi = mesh.vertex_indices[ci] as usize;
                let (ji, jw) = skin_weights(skin, vi, &sd.cluster_to_bone);
                let idx = verts.len() as u32;
                verts.push(SkinnedVertex {
                    position: [p.x as f32, p.y as f32, p.z as f32],
                    normal: if hn { let n = &mesh.vertex_normal[ci]; [n.x as f32, n.y as f32, n.z as f32] } else { [0.0, 1.0, 0.0] },
                    uv: if hu { let u = &mesh.vertex_uv[ci]; [u.x as f32, u.y as f32] } else { [0.0, 0.0] },
                    tangent: if ht { let t = &mesh.vertex_tangent[ci]; [t.x as f32, t.y as f32, t.z as f32, 1.0] } else { [0.0, 0.0, 0.0, 1.0] },
                    joint_indices: ji,
                    joint_weights: jw,
                });
                idxs.push(idx);
            }
        }
    }
    (verts, idxs)
}

fn skin_weights(
    skin: &ufbx::SkinDeformer,
    vertex: usize,
    cluster_to_bone: &HashMap<usize, usize>,
) -> ([u16; 4], [f32; 4]) {
    let mut ji = [0u16; 4];
    let mut jw = [0.0f32; 4];

    if vertex >= skin.vertices.len() {
        jw[0] = 1.0;
        return (ji, jw);
    }

    let sv = &skin.vertices[vertex];
    let begin = sv.weight_begin as usize;
    let count = (sv.num_weights as usize).min(skin.weights.len().saturating_sub(begin));

    let mut pairs: Vec<(u16, f32)> = (0..count)
        .map(|i| {
            let w = &skin.weights[begin + i];
            let bone = cluster_to_bone
                .get(&(w.cluster_index as usize))
                .copied()
                .unwrap_or(0) as u16;
            (bone, w.weight as f32)
        })
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let take = pairs.len().min(4);
    let mut total = 0.0f32;
    for i in 0..take {
        ji[i] = pairs[i].0;
        jw[i] = pairs[i].1;
        total += jw[i];
    }
    if total > 0.0 {
        for w in &mut jw[..take] { *w /= total; }
    } else {
        jw[0] = 1.0;
    }
    (ji, jw)
}

// ---- Material ----

fn extract_material(mesh: &ufbx::Mesh, tex_map: &HashMap<u32, usize>) -> Option<GltfLoadedMaterial> {
    if mesh.materials.is_empty() {
        return None;
    }
    let mat = &mesh.materials[0];
    let mut result = GltfLoadedMaterial {
        albedo: Vec3::new(0.8, 0.8, 0.8),
        roughness: 0.5,
        metallic: 0.0,
        emission: Vec3::ZERO,
        albedo_texture: None,
        normal_texture: None,
    };

    for ti in 0..mat.textures.len() {
        let mt = &mat.textures[ti];
        let prop: &str = &mt.material_prop;
        let tex_id = mt.texture.element.typed_id;

        if prop.contains("Diffuse") || prop.contains("base_color") || prop.contains("BaseColor") || prop.contains("albedo") {
            if let Some(&idx) = tex_map.get(&tex_id) {
                result.albedo_texture = Some(idx);
            }
        }
        if prop.contains("Normal") || prop.contains("normal") || prop.contains("Bump") {
            if let Some(&idx) = tex_map.get(&tex_id) {
                result.normal_texture = Some(idx);
            }
        }
    }
    Some(result)
}

// ---- Animation ----

fn extract_animations(sc: &ufbx::Scene, skin_data: &[SkinData]) -> Vec<AnimationClip> {
    let mut clips = Vec::new();

    if sc.anim_stacks.is_empty() {
        log::warn!("[FBX:Anim] No anim_stacks found — file contains no animations");
        return clips;
    }
    log::info!("[FBX:Anim] Found {} anim_stack(s), {} skeleton(s) for bone mapping",
        sc.anim_stacks.len(), skin_data.len());

    // Build name-to-bone fallback mapping (for nodes not in skin clusters)
    let name_to_bone: HashMap<String, usize> = skin_data
        .iter()
        .flat_map(|sd| {
            sd.skeleton.bones.iter().enumerate().map(|(i, b)| (b.name.clone(), i))
        })
        .collect();

    for i in 0..sc.anim_stacks.len() {
        let stack = &sc.anim_stacks[i];
        let stack_name: &str = &stack.element.name;
        log::debug!("[FBX:Anim] Baking anim stack {} '{}' at 30fps", i, stack_name);
        let bake_opts = ufbx::BakeOpts {
            resample_rate: 30.0,
            ..Default::default()
        };

        let bake_start = std::time::Instant::now();
        let baked = match ufbx::bake_anim(sc, &stack.anim, bake_opts) {
            Ok(b) => b,
            Err(e) => {
                log::warn!("[FBX:Anim] Failed to bake anim stack {} '{}': {:?}", i, stack_name, e);
                continue;
            }
        };
        log::debug!("[FBX:Anim] Baked '{}' in {:.1}ms — {} animated nodes",
            stack_name, bake_start.elapsed().as_secs_f64() * 1000.0, baked.nodes.len());

        let mut channels = Vec::new();
        let mut max_time: f32 = 0.0;
        let mut mapped_count = 0u32;
        let mut unmapped_count = 0u32;

        for bn in baked.nodes.iter() {
            // Primary: lookup by typed_id in skin data
            let bone_idx = skin_data
                .iter()
                .find_map(|sd| sd.node_to_bone.get(&bn.typed_id).copied())
                .or_else(|| {
                    // Fallback: lookup by node name (Mixamo root nodes, etc.)
                    let nid = bn.typed_id as usize;
                    if nid < sc.nodes.len() {
                        let node_name: &str = &sc.nodes[nid].element.name;
                        name_to_bone.get(node_name).copied()
                    } else {
                        None
                    }
                });

            let bone_idx = match bone_idx {
                Some(bi) => {
                    mapped_count += 1;
                    bi
                }
                None => {
                    unmapped_count += 1;
                    let nid = bn.typed_id as usize;
                    let node_name = if nid < sc.nodes.len() {
                        sc.nodes[nid].element.name.to_string()
                    } else {
                        format!("node_{}", bn.typed_id)
                    };
                    log::trace!("[FBX:Anim] Node '{}' (typed_id={}) not mapped to any bone, skipping", node_name, bn.typed_id);
                    continue;
                }
            };

            if !bn.translation_keys.is_empty() {
                let ts: Vec<f32> = bn.translation_keys.iter().map(|k| k.time as f32).collect();
                let vs: Vec<f32> = bn.translation_keys.iter()
                    .flat_map(|k| [k.value.x as f32, k.value.y as f32, k.value.z as f32])
                    .collect();
                max_time = max_time.max(*ts.last().unwrap_or(&0.0));
                channels.push(AnimationChannel {
                    target_bone: bone_idx, property: AnimationProperty::Translation,
                    interpolation: InterpolationMode::Linear, timestamps: ts, values: vs,
                });
            }

            if !bn.rotation_keys.is_empty() {
                let ts: Vec<f32> = bn.rotation_keys.iter().map(|k| k.time as f32).collect();
                let vs: Vec<f32> = bn.rotation_keys.iter()
                    .flat_map(|k| [k.value.x as f32, k.value.y as f32, k.value.z as f32, k.value.w as f32])
                    .collect();
                max_time = max_time.max(*ts.last().unwrap_or(&0.0));
                channels.push(AnimationChannel {
                    target_bone: bone_idx, property: AnimationProperty::Rotation,
                    interpolation: InterpolationMode::Linear, timestamps: ts, values: vs,
                });
            }

            if !bn.scale_keys.is_empty() {
                let ts: Vec<f32> = bn.scale_keys.iter().map(|k| k.time as f32).collect();
                let vs: Vec<f32> = bn.scale_keys.iter()
                    .flat_map(|k| [k.value.x as f32, k.value.y as f32, k.value.z as f32])
                    .collect();
                max_time = max_time.max(*ts.last().unwrap_or(&0.0));
                channels.push(AnimationChannel {
                    target_bone: bone_idx, property: AnimationProperty::Scale,
                    interpolation: InterpolationMode::Linear, timestamps: ts, values: vs,
                });
            }
        }

        let anim_name = {
            let n: &str = &stack.element.name;
            if n.is_empty() { "Animation".to_string() } else { n.to_owned() }
        };
        log::debug!("[FBX:Anim] '{}': {} nodes mapped, {} unmapped", anim_name, mapped_count, unmapped_count);
        if channels.is_empty() {
            log::warn!("[FBX:Anim] '{}': 0 channels extracted (all {} nodes unmatched) — animation will be empty", anim_name, unmapped_count);
            continue;
        }
        log::info!("[FBX:Anim] '{}': {} channels, {:.2}s duration, {}/{} nodes mapped",
            anim_name, channels.len(), max_time, mapped_count, mapped_count + unmapped_count);
        clips.push(AnimationClip { name: anim_name, duration: max_time, channels });
    }
    clips
}

// ---- Textures ----

fn extract_textures(sc: &ufbx::Scene, fbx_path: &Path) -> Vec<GltfLoadedTexture> {
    let fbx_dir = fbx_path.parent();
    let path_str = fbx_path.to_string_lossy();
    let mut textures = Vec::new();

    for i in 0..sc.textures.len() {
        let tex = &sc.textures[i];
        let cache_key = format!("fbx:{}#tex_{}", path_str, i);

        // Try embedded content
        if !tex.content.is_empty() {
            if let Some(loaded) = load_tex_bytes(&tex.content, &cache_key) {
                textures.push(loaded);
                continue;
            }
        }

        // Try relative path
        let rel: &str = &tex.relative_filename;
        if !rel.is_empty() {
            if let Some(dir) = fbx_dir {
                let p = dir.join(rel);
                if p.exists() {
                    if let Some(loaded) = load_tex_file(&p, &cache_key) {
                        textures.push(loaded);
                        continue;
                    }
                }
            }
        }

        // Try absolute path
        let abs: &str = &tex.absolute_filename;
        if !abs.is_empty() {
            let p = Path::new(abs);
            if p.exists() {
                if let Some(loaded) = load_tex_file(p, &cache_key) {
                    textures.push(loaded);
                    continue;
                }
            }
        }

        log::debug!("FBX texture {} not found: {}", i, rel);
    }
    textures
}

fn load_tex_bytes(data: &[u8], cache_key: &str) -> Option<GltfLoadedTexture> {
    let img = image::load_from_memory(data).ok()?;
    let rgba = img.to_rgba8();
    Some(GltfLoadedTexture {
        cache_key: cache_key.to_string(),
        rgba: rgba.to_vec(),
        width: rgba.width(),
        height: rgba.height(),
    })
}

fn load_tex_file(path: &Path, cache_key: &str) -> Option<GltfLoadedTexture> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    Some(GltfLoadedTexture {
        cache_key: cache_key.to_string(),
        rgba: rgba.to_vec(),
        width: rgba.width(),
        height: rgba.height(),
    })
}
