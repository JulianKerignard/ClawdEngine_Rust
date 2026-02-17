use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};

use crate::core::{
    AnimationChannel, AnimationClip, AnimationProperty, Bone, InterpolationMode, Skeleton,
    Transform,
};
use crate::renderer::mesh::{compute_tangents, Vertex};
use crate::renderer::skinned_mesh::SkinnedVertex;

// ---- Output structures ----

pub struct GltfScene {
    pub meshes: Vec<GltfLoadedMesh>,
    pub textures: Vec<GltfLoadedTexture>,
    pub skeletons: Vec<Skeleton>,
    pub skinned_meshes: Vec<GltfLoadedSkinnedMesh>,
    pub animation_clips: Vec<AnimationClip>,
}

pub struct GltfLoadedMesh {
    pub store_name: String,
    pub display_name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
    pub material: Option<GltfLoadedMaterial>,
}

pub struct GltfLoadedMaterial {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub emission: Vec3,
    pub albedo_texture: Option<usize>,
    pub normal_texture: Option<usize>,
}

pub struct GltfLoadedTexture {
    pub cache_key: String,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct GltfLoadedSkinnedMesh {
    pub store_name: String,
    pub display_name: String,
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
    pub material: Option<GltfLoadedMaterial>,
    pub skeleton_index: usize, // index into GltfScene.skeletons
}

// ---- Skin extraction helpers ----

struct SkinData {
    skeleton: Skeleton,
    node_to_joint: HashMap<usize, usize>, // glTF node index → bone index in skeleton (post-sort)
}

fn build_parent_map(document: &gltf::Document) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    fn visit(node: &gltf::Node, map: &mut HashMap<usize, usize>) {
        for child in node.children() {
            map.insert(child.index(), node.index());
            visit(&child, map);
        }
    }
    for gltf_scene in document.scenes() {
        for node in gltf_scene.nodes() {
            visit(&node, &mut map);
        }
    }
    map
}

fn extract_skin(
    skin: &gltf::Skin,
    buffers: &[gltf::buffer::Data],
    parent_map: &HashMap<usize, usize>,
) -> SkinData {
    let joints: Vec<gltf::Node> = skin.joints().collect();
    let joint_count = joints.len();

    // Read inverse bind matrices (or default to identity)
    let reader = skin.reader(|buf| Some(&buffers[buf.index()]));
    let ibms: Vec<Mat4> = reader
        .read_inverse_bind_matrices()
        .map(|iter| iter.map(|m| Mat4::from_cols_array_2d(&m)).collect())
        .unwrap_or_else(|| vec![Mat4::IDENTITY; joint_count]);

    // Build node_to_joint mapping
    let node_to_joint: HashMap<usize, usize> = joints
        .iter()
        .enumerate()
        .map(|(ji, node)| (node.index(), ji))
        .collect();

    // Build unsorted bones: (original_joint_index, Bone)
    let mut unsorted: Vec<(usize, Bone)> = Vec::with_capacity(joint_count);
    for (ji, joint_node) in joints.iter().enumerate() {
        let parent_joint = parent_map
            .get(&joint_node.index())
            .and_then(|&pn| node_to_joint.get(&pn).copied());

        let (translation, rotation, scale) = joint_node.transform().decomposed();
        let bone = Bone {
            name: joint_node
                .name()
                .unwrap_or(&format!("bone_{}", ji))
                .to_string(),
            parent: parent_joint,
            children: Vec::new(), // filled after sort
            inverse_bind_matrix: ibms[ji],
            local_bind_transform: Transform {
                position: Vec3::from(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from(scale),
            },
        };
        unsorted.push((ji, bone));
    }

    // Topological sort + remap
    let (sorted_bones, old_to_new) = topological_sort_bones(unsorted);

    // Remap node_to_joint to use post-sort bone indices
    let node_to_joint: HashMap<usize, usize> = node_to_joint
        .into_iter()
        .filter_map(|(node_idx, old_ji)| {
            old_to_new.get(&old_ji).map(|&new_ji| (node_idx, new_ji))
        })
        .collect();

    let root_bone = sorted_bones
        .iter()
        .position(|b| b.parent.is_none())
        .unwrap_or(0);
    let skel_name = skin.name().unwrap_or("skeleton").to_string();

    SkinData {
        skeleton: Skeleton::new(skel_name, sorted_bones, root_bone),
        node_to_joint,
    }
}

pub(crate) fn topological_sort_bones(unsorted: Vec<(usize, Bone)>) -> (Vec<Bone>, HashMap<usize, usize>) {
    let count = unsorted.len();
    // BFS from roots
    let mut order: Vec<usize> = Vec::with_capacity(count); // indices into unsorted
    let mut queue = std::collections::VecDeque::new();

    // Find roots (no parent)
    for (pos, (_, bone)) in unsorted.iter().enumerate() {
        if bone.parent.is_none() {
            queue.push_back(pos);
        }
    }

    while let Some(pos) = queue.pop_front() {
        order.push(pos);
        let old_idx = unsorted[pos].0;
        // Find children (bones whose parent == old_idx)
        for (child_pos, (_, bone)) in unsorted.iter().enumerate() {
            if bone.parent == Some(old_idx) {
                queue.push_back(child_pos);
            }
        }
    }

    // Add any orphaned bones not reached by BFS
    if order.len() < count {
        for pos in 0..count {
            if !order.contains(&pos) {
                order.push(pos);
            }
        }
    }

    // Build remap: old_joint_index → new_position
    let mut old_to_new: HashMap<usize, usize> = HashMap::new();
    for (new_pos, &unsorted_pos) in order.iter().enumerate() {
        let old_idx = unsorted[unsorted_pos].0;
        old_to_new.insert(old_idx, new_pos);
    }

    // Build final bones with remapped parent/children
    let mut result: Vec<Bone> = order
        .iter()
        .map(|&pos| {
            let (_, bone) = &unsorted[pos];
            let new_parent = bone.parent.and_then(|p| old_to_new.get(&p).copied());
            Bone {
                name: bone.name.clone(),
                parent: new_parent,
                children: Vec::new(),
                inverse_bind_matrix: bone.inverse_bind_matrix,
                local_bind_transform: bone.local_bind_transform,
            }
        })
        .collect();

    // Fill children arrays
    for i in 0..result.len() {
        if let Some(pi) = result[i].parent {
            // Safety: pi < i guaranteed by topological sort
            let child_idx = i;
            result[pi].children.push(child_idx);
        }
    }

    (result, old_to_new)
}

fn extract_skinned_primitive(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Option<(Vec<SkinnedVertex>, Vec<u32>)> {
    let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let vertex_count = positions.len();

    let normals: Vec<[f32; 3]> = reader
        .read_normals()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; vertex_count]);

    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|tc| tc.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; vertex_count]);

    let tangents: Vec<[f32; 4]> = reader
        .read_tangents()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 0.0, 0.0, 1.0]; vertex_count]);

    let joints: Vec<[u16; 4]> = reader.read_joints(0)?.into_u16().collect();

    let weights: Vec<[f32; 4]> = reader
        .read_weights(0)
        .map(|w| w.into_f32().collect())
        .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; vertex_count]);

    let indices: Vec<u32> = reader
        .read_indices()
        .map(|idx| idx.into_u32().collect())
        .unwrap_or_else(|| (0..vertex_count as u32).collect());

    let vertices: Vec<SkinnedVertex> = (0..vertex_count)
        .map(|i| SkinnedVertex {
            position: positions[i],
            normal: normals[i],
            uv: uvs[i],
            tangent: tangents[i],
            joint_indices: joints[i],
            joint_weights: weights[i],
        })
        .collect();

    Some((vertices, indices))
}

// ---- Animation extraction ----

fn extract_animations(
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    skin_data: &[SkinData],
) -> Vec<AnimationClip> {
    let mut clips = Vec::new();
    let anim_count = document.animations().count();
    if anim_count == 0 {
        log::debug!("[glTF:Anim] No animations in document");
        return clips;
    }
    log::info!("[glTF:Anim] Extracting {} animation(s)", anim_count);

    for animation in document.animations() {
        let anim_name = animation.name().unwrap_or("unnamed");
        let channel_count = animation.channels().count();
        log::debug!("[glTF:Anim] Processing '{}': {} channels", anim_name, channel_count);

        let mut channels = Vec::new();
        let mut max_time: f32 = 0.0;
        let mut unmapped = 0u32;

        for channel in animation.channels() {
            let target = channel.target();
            let node_idx = target.node().index();

            // Find which skin contains this node → get remapped bone index
            let bone_idx = skin_data
                .iter()
                .find_map(|sd| sd.node_to_joint.get(&node_idx).copied());

            let bone_idx = match bone_idx {
                Some(bi) => bi,
                None => {
                    unmapped += 1;
                    continue;
                }
            };

            let property = match target.property() {
                gltf::animation::Property::Translation => AnimationProperty::Translation,
                gltf::animation::Property::Rotation => AnimationProperty::Rotation,
                gltf::animation::Property::Scale => AnimationProperty::Scale,
                _ => continue, // MorphTargetWeights: skip for MVP
            };

            let interpolation = match channel.sampler().interpolation() {
                gltf::animation::Interpolation::Linear => InterpolationMode::Linear,
                gltf::animation::Interpolation::Step => InterpolationMode::Step,
                gltf::animation::Interpolation::CubicSpline => InterpolationMode::CubicSpline,
            };

            let reader = channel.reader(|buf| Some(&buffers[buf.index()]));

            let timestamps: Vec<f32> = match reader.read_inputs() {
                Some(iter) => iter.collect(),
                None => continue,
            };

            max_time = max_time.max(timestamps.last().copied().unwrap_or(0.0));

            let values: Vec<f32> = match reader.read_outputs() {
                Some(outputs) => match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => {
                        iter.flatten().collect()
                    }
                    gltf::animation::util::ReadOutputs::Rotations(iter) => {
                        iter.into_f32().flatten().collect()
                    }
                    gltf::animation::util::ReadOutputs::Scales(iter) => {
                        iter.flatten().collect()
                    }
                    _ => continue,
                },
                None => continue,
            };

            channels.push(AnimationChannel {
                target_bone: bone_idx,
                property,
                interpolation,
                timestamps,
                values,
            });
        }

        if unmapped > 0 {
            log::debug!("[glTF:Anim] '{}': {} channel targets not in any skeleton", anim_name, unmapped);
        }
        if channels.is_empty() {
            log::warn!("[glTF:Anim] '{}': 0 usable channels after bone mapping", anim_name);
            continue;
        }

        log::info!("[glTF:Anim] '{}': {} channels, {:.2}s duration", anim_name, channels.len(), max_time);
        clips.push(AnimationClip {
            name: animation.name().unwrap_or("Animation").to_string(),
            duration: max_time,
            channels,
        });
    }

    clips
}

// ---- Public API ----

pub fn load_gltf(path: impl AsRef<Path>) -> Result<GltfScene> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    log::info!("[glTF] Loading file: {}", path_str);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    log::debug!("[glTF] File size: {} bytes ({:.1} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));

    let parse_start = std::time::Instant::now();
    let (document, buffers, images) =
        gltf::import(path).with_context(|| format!("Failed to import glTF: {}", path_str))?;
    log::debug!("[glTF] Parsed in {:.1}ms — {} buffers, {} images",
        parse_start.elapsed().as_secs_f64() * 1000.0, buffers.len(), images.len());

    let textures = extract_textures(&images, &path_str);

    let mut scene = GltfScene {
        meshes: Vec::new(),
        textures,
        skeletons: Vec::new(),
        skinned_meshes: Vec::new(),
        animation_clips: Vec::new(),
    };

    // Build a texture index map: gltf texture index → our textures vec index
    let tex_index_map: Vec<Option<usize>> = document
        .textures()
        .map(|tex| {
            let img_idx = tex.source().index();
            if img_idx < scene.textures.len() {
                Some(img_idx)
            } else {
                None
            }
        })
        .collect();

    // Extract skins → skeletons + node_to_joint mappings
    let parent_map = build_parent_map(&document);
    let skin_count = document.skins().count();
    log::debug!("[glTF] Found {} skin(s), {} scene(s), {} animation(s)",
        skin_count, document.scenes().count(), document.animations().count());
    let skin_data: Vec<SkinData> = document
        .skins()
        .enumerate()
        .map(|(i, skin)| {
            let sd = extract_skin(&skin, &buffers, &parent_map);
            log::debug!("[glTF:Skin] Skin {} '{}': {} bones, {} node-to-joint mappings",
                i, sd.skeleton.name, sd.skeleton.bones.len(), sd.node_to_joint.len());
            sd
        })
        .collect();
    if skin_data.is_empty() {
        log::debug!("[glTF] No skins found — file has no skeletal data");
    }

    // Walk the scene graph (flatten hierarchy)
    for gltf_scene in document.scenes() {
        for node in gltf_scene.nodes() {
            visit_node(&node, Mat4::IDENTITY, &buffers, &tex_index_map, &path_str, &skin_data, &mut scene);
        }
    }

    // Extract animations (needs &skin_data, which is consumed below)
    scene.animation_clips = extract_animations(&document, &buffers, &skin_data);

    // Move skeletons from SkinData into scene output
    for sd in skin_data {
        scene.skeletons.push(sd.skeleton);
    }

    log::info!(
        "Loaded glTF: {} ({} meshes, {} skinned, {} skeletons, {} animations, {} textures)",
        path_str,
        scene.meshes.len(),
        scene.skinned_meshes.len(),
        scene.skeletons.len(),
        scene.animation_clips.len(),
        scene.textures.len()
    );

    Ok(scene)
}

// ---- Node traversal ----

fn visit_node(
    node: &gltf::Node,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    tex_index_map: &[Option<usize>],
    file_path: &str,
    skin_data: &[SkinData],
    scene: &mut GltfScene,
) {
    let local_transform = node_transform(node);
    let world_transform = parent_transform * local_transform;

    if let Some(mesh) = node.mesh() {
        let node_name = node
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("node_{}", node.index()));

        // Check if this node has a skin (→ skinned mesh path)
        let skin_idx = node.skin().map(|s| s.index());

        for (prim_idx, primitive) in mesh.primitives().enumerate() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                log::warn!(
                    "Skipping non-triangle primitive in {}/{} (mode: {:?})",
                    node_name,
                    prim_idx,
                    primitive.mode()
                );
                continue;
            }

            let material = convert_material(&primitive.material(), tex_index_map);
            let store_name = format!("gltf:{}#{}/{}", file_path, node_name, prim_idx);
            let display_name = if mesh.primitives().len() > 1 {
                format!("{}.{}", node_name, prim_idx)
            } else {
                node_name.clone()
            };

            // Skinned mesh path: extract joints/weights
            if let Some(si) = skin_idx {
                if si < skin_data.len() {
                    if let Some((vertices, indices)) =
                        extract_skinned_primitive(&primitive, buffers)
                    {
                        log::debug!("[glTF:Mesh] '{}' → skinned ({} verts, {} idx, skin={})",
                            display_name, vertices.len(), indices.len(), si);
                        scene.skinned_meshes.push(GltfLoadedSkinnedMesh {
                            store_name,
                            display_name,
                            vertices,
                            indices,
                            transform: world_transform,
                            material: Some(material),
                            skeleton_index: si,
                        });
                        continue;
                    }
                    log::debug!("[glTF:Mesh] '{}': has skin but missing JOINTS_0, falling back to static mesh", display_name);
                    // Fallthrough: JOINTS_0 missing → treat as normal mesh
                }
            }

            // Normal mesh path
            if let Some((mut vertices, indices)) = extract_primitive(&primitive, buffers) {
                let has_tangents = vertices.iter().any(|v| {
                    v.tangent[0] != 0.0 || v.tangent[1] != 0.0 || v.tangent[2] != 0.0
                });
                if !has_tangents {
                    compute_tangents(&mut vertices, &indices);
                }

                scene.meshes.push(GltfLoadedMesh {
                    store_name,
                    display_name,
                    vertices,
                    indices,
                    transform: world_transform,
                    material: Some(material),
                });
            }
        }
    }

    // Recurse into children
    for child in node.children() {
        visit_node(&child, world_transform, buffers, tex_index_map, file_path, skin_data, scene);
    }
}

fn node_transform(node: &gltf::Node) -> Mat4 {
    let (translation, rotation, scale) = node.transform().decomposed();
    let t = Vec3::from(translation);
    let r = Quat::from_array(rotation);
    let s = Vec3::from(scale);
    Mat4::from_scale_rotation_translation(s, r, t)
}

// ---- Primitive extraction ----

fn extract_primitive(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Option<(Vec<Vertex>, Vec<u32>)> {
    let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let vertex_count = positions.len();

    let normals: Vec<[f32; 3]> = reader
        .read_normals()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; vertex_count]);

    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|tc| tc.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; vertex_count]);

    let tangents: Vec<[f32; 4]> = reader
        .read_tangents()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 0.0, 0.0, 1.0]; vertex_count]);

    let indices: Vec<u32> = reader
        .read_indices()
        .map(|idx| idx.into_u32().collect())
        .unwrap_or_else(|| (0..vertex_count as u32).collect());

    let vertices: Vec<Vertex> = (0..vertex_count)
        .map(|i| Vertex {
            position: positions[i],
            normal: normals[i],
            uv: uvs[i],
            tangent: tangents[i],
        })
        .collect();

    Some((vertices, indices))
}

// ---- Material conversion ----

fn convert_material(
    gltf_mat: &gltf::Material,
    tex_index_map: &[Option<usize>],
) -> GltfLoadedMaterial {
    let pbr = gltf_mat.pbr_metallic_roughness();
    let base = pbr.base_color_factor();
    // Only use emissive_factor if there's no emissive texture
    // (our shader doesn't support emissive textures, so [1,1,1] would blow out the image)
    let emission = if gltf_mat.emissive_texture().is_some() {
        Vec3::ZERO
    } else {
        Vec3::from(gltf_mat.emissive_factor())
    };

    let albedo_texture = pbr
        .base_color_texture()
        .and_then(|info| tex_index_map.get(info.texture().index()).copied().flatten());

    let normal_texture = gltf_mat
        .normal_texture()
        .and_then(|info| tex_index_map.get(info.texture().index()).copied().flatten());

    GltfLoadedMaterial {
        albedo: Vec3::new(base[0], base[1], base[2]),
        roughness: pbr.roughness_factor(),
        metallic: pbr.metallic_factor(),
        emission,
        albedo_texture,
        normal_texture,
    }
}

// ---- Texture extraction ----

fn extract_textures(images: &[gltf::image::Data], file_path: &str) -> Vec<GltfLoadedTexture> {
    images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            log::info!(
                "glTF texture {}: {}x{}, format={:?}, {} bytes",
                i, img.width, img.height, img.format, img.pixels.len()
            );
            let rgba = match img.format {
                gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
                gltf::image::Format::R8G8B8 => rgb_to_rgba(&img.pixels),
                gltf::image::Format::R16G16B16A16 => rgba16_to_rgba8(&img.pixels),
                gltf::image::Format::R16G16B16 => rgb16_to_rgba8(&img.pixels),
                gltf::image::Format::R8 => r8_to_rgba(&img.pixels),
                gltf::image::Format::R16 => r16_to_rgba(&img.pixels),
                gltf::image::Format::R8G8 => rg8_to_rgba(&img.pixels),
                gltf::image::Format::R16G16 => rg16_to_rgba(&img.pixels),
                gltf::image::Format::R32G32B32FLOAT => rgb32f_to_rgba(&img.pixels),
                gltf::image::Format::R32G32B32A32FLOAT => rgba32f_to_rgba(&img.pixels),
            };

            GltfLoadedTexture {
                cache_key: format!("gltf-embedded:{}#texture_{}", file_path, i),
                rgba,
                width: img.width,
                height: img.height,
            }
        })
        .collect()
}

// ---- Format conversion helpers ----

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let pixel_count = rgb.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.extend_from_slice(chunk);
        rgba.push(255);
    }
    rgba
}

fn rgba16_to_rgba8(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(2)
        .map(|c| (u16::from_le_bytes([c[0], c[1]]) >> 8) as u8)
        .collect()
}

fn rgb16_to_rgba8(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 6;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(6) {
        for ch in pixel.chunks_exact(2) {
            rgba.push((u16::from_le_bytes([ch[0], ch[1]]) >> 8) as u8);
        }
        rgba.push(255);
    }
    rgba
}

fn r8_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 4);
    for &v in data {
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
}

fn r16_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 2);
    for chunk in data.chunks_exact(2) {
        let v = (u16::from_le_bytes([chunk[0], chunk[1]]) >> 8) as u8;
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
}

fn rg8_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 2);
    for chunk in data.chunks_exact(2) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], 0, 255]);
    }
    rgba
}

fn rg16_to_rgba(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 4;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(4) {
        let r = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
        let g = (u16::from_le_bytes([pixel[2], pixel[3]]) >> 8) as u8;
        rgba.extend_from_slice(&[r, g, 0, 255]);
    }
    rgba
}

fn rgb32f_to_rgba(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 12;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(12) {
        for ch in pixel.chunks_exact(4) {
            let f = f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]);
            rgba.push((f.clamp(0.0, 1.0) * 255.0) as u8);
        }
        rgba.push(255);
    }
    rgba
}

fn rgba32f_to_rgba(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(4)
        .map(|ch| {
            let f = f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]);
            (f.clamp(0.0, 1.0) * 255.0) as u8
        })
        .collect()
}
