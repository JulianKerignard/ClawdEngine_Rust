use std::sync::mpsc;

use crate::assets::gltf_loader::{GltfLoadedMesh, GltfLoadedSkinnedMesh, GltfLoadedTexture};
use crate::core::{self, AnimationClip, Skeleton, SkeletalAnimator, World};
use crate::renderer::{GpuContext, SceneRenderer};

use super::context::EditorContext;

// ---- Types ----

pub struct PrecomputedTexture {
    pub cache_key: String,
    pub width: u32,
    pub height: u32,
    pub mip_levels: Vec<Vec<u8>>,
}

pub struct PreparedGltfAsset {
    pub meshes: Vec<GltfLoadedMesh>,
    pub textures: Vec<PrecomputedTexture>,
    pub skeletons: Vec<Skeleton>,
    pub skinned_meshes: Vec<GltfLoadedSkinnedMesh>,
    pub animation_clips: Vec<AnimationClip>,
    pub asset_path: String,
}

pub enum AssetLoadState {
    Parsing {
        receiver: mpsc::Receiver<Result<PreparedGltfAsset, String>>,
        drop_pos: glam::Vec3,
    },
    Finalizing {
        prepared: PreparedGltfAsset,
        drop_pos: glam::Vec3,
    },
}

// ---- Background loading ----

fn mip_level_count(w: u32, h: u32) -> u32 {
    (w.max(h) as f32).log2().floor() as u32 + 1
}

fn precompute_mipmaps(tex: GltfLoadedTexture) -> PrecomputedTexture {
    let mip_count = mip_level_count(tex.width, tex.height);
    let mut mip_levels = Vec::with_capacity(mip_count as usize);

    if mip_count > 1 {
        // from_raw consumes the Vec — on success we get the ImageBuffer,
        // on failure (wrong size) the data is returned via into_raw()
        match image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
            tex.width,
            tex.height,
            tex.rgba,
        ) {
            Some(mut current) => {
                mip_levels.push(current.as_raw().clone());
                for level in 1..mip_count {
                    let mip_w = (tex.width >> level).max(1);
                    let mip_h = (tex.height >> level).max(1);
                    current = image::imageops::resize(
                        &current,
                        mip_w,
                        mip_h,
                        image::imageops::FilterType::Triangle,
                    );
                    mip_levels.push(current.to_vec());
                }
            }
            None => {
                // Should not happen, but fallback: single mip with empty data
                mip_levels.push(vec![128; (tex.width * tex.height * 4) as usize]);
            }
        }
    } else {
        mip_levels.push(tex.rgba);
    }

    PrecomputedTexture {
        cache_key: tex.cache_key,
        width: tex.width,
        height: tex.height,
        mip_levels,
    }
}

pub fn start_background_load(
    asset_path: &str,
) -> mpsc::Receiver<Result<PreparedGltfAsset, String>> {
    let (tx, rx) = mpsc::channel();
    let path = asset_path.to_string();

    log::info!("[AssetLoader] Starting background load: {}", asset_path);
    std::thread::spawn(move || {
        let load_start = std::time::Instant::now();
        let result = (|| -> Result<PreparedGltfAsset, String> {
            let loader_type = if path.ends_with(".fbx") { "FBX" } else { "glTF" };
            log::debug!("[AssetLoader:BG] Parsing {} file: {}", loader_type, path);
            let gltf_scene = if path.ends_with(".fbx") {
                crate::assets::fbx_loader::load_fbx(&path).map_err(|e| e.to_string())?
            } else {
                crate::assets::gltf_loader::load_gltf(&path).map_err(|e| e.to_string())?
            };

            let textures: Vec<PrecomputedTexture> = gltf_scene
                .textures
                .into_iter()
                .map(precompute_mipmaps)
                .collect();

            log::debug!("[AssetLoader:BG] Precomputed {} texture mipmaps", textures.len());
            Ok(PreparedGltfAsset {
                meshes: gltf_scene.meshes,
                textures,
                skeletons: gltf_scene.skeletons,
                skinned_meshes: gltf_scene.skinned_meshes,
                animation_clips: gltf_scene.animation_clips,
                asset_path: path,
            })
        })();

        match &result {
            Ok(asset) => log::info!("[AssetLoader:BG] Prepared in {:.0}ms: {} meshes, {} skinned, {} skels, {} anims",
                load_start.elapsed().as_secs_f64() * 1000.0,
                asset.meshes.len(), asset.skinned_meshes.len(), asset.skeletons.len(), asset.animation_clips.len()),
            Err(e) => log::error!("[AssetLoader:BG] Failed after {:.0}ms: {}", load_start.elapsed().as_secs_f64() * 1000.0, e),
        }
        let _ = tx.send(result);
    });

    rx
}

// ---- Per-frame tick ----

pub fn tick_loading(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
) {
    let state = match ec.asset_load_state.take() {
        Some(s) => s,
        None => return,
    };

    match state {
        AssetLoadState::Parsing { receiver, drop_pos } => match receiver.try_recv() {
            Ok(Ok(prepared)) => {
                log::info!("[AssetLoader] Background parse complete — transitioning to GPU upload phase");
                ec.asset_load_state = Some(AssetLoadState::Finalizing { prepared, drop_pos });
            }
            Ok(Err(e)) => {
                log::error!("[AssetLoader] Background load FAILED: {}", e);
                ec.loading_status = None;
                ec.save_feedback = Some((format!("Load failed: {}", e), 3.0));
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Still parsing on background thread
                ec.asset_load_state = Some(AssetLoadState::Parsing { receiver, drop_pos });
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                log::error!("[AssetLoader] Background loader thread panicked or disconnected");
                ec.loading_status = None;
            }
        },

        AssetLoadState::Finalizing { prepared, drop_pos } => {
            log::debug!("[AssetLoader] Finalizing: uploading {} textures to GPU", prepared.textures.len());
            // Upload ALL textures in one frame (write_texture is async, <1ms each)
            let tex_ids: Vec<usize> = prepared
                .textures
                .iter()
                .map(|tex| {
                    scene.texture_store.upload_precomputed(
                        &gpu.device,
                        &gpu.queue,
                        &tex.cache_key,
                        tex.width,
                        tex.height,
                        &tex.mip_levels,
                    )
                })
                .collect();

            create_entities_from_prepared(world, ec, scene, gpu, &prepared, &tex_ids, drop_pos);
            ec.loading_status = None;
        }
    }
}

fn create_entities_from_prepared(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
    prepared: &PreparedGltfAsset,
    tex_ids: &[usize],
    drop_pos: glam::Vec3,
) {
    let mut drop_pos = drop_pos;

    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    // Extract sub-assets to disk (skeleton, clips, materials, textures)
    if let Err(e) = crate::assets::extracted_assets::extract_subassets_from_prepared(prepared) {
        log::warn!("[AssetLoader] Sub-asset extraction failed: {}", e);
    }

    log::info!("[AssetLoader] Creating entities from '{}' at position {:?}", prepared.asset_path, drop_pos);

    // Store skeletons first (we need IDs for SkeletalAnimator)
    let skeleton_ids: Vec<usize> = prepared
        .skeletons
        .iter()
        .enumerate()
        .map(|(i, skel)| {
            if let Some(id) = scene.skeleton_store.find_by_name(&skel.name) {
                log::debug!("[AssetLoader] Skeleton '{}' already in store (id={})", skel.name, id);
                id
            } else {
                let id = scene.skeleton_store.add(skel.clone());
                log::debug!("[AssetLoader] Stored skeleton {} '{}' → store id {}, {} bones",
                    i, skel.name, id, skel.bones.len());
                id
            }
        })
        .collect();

    // Store animation clips
    let clip_ids: Vec<usize> = prepared
        .animation_clips
        .iter()
        .enumerate()
        .map(|(i, clip)| {
            let id = scene
                .animation_clip_store
                .find_by_name(&clip.name)
                .unwrap_or_else(|| scene.animation_clip_store.add(clip.clone()));
            log::debug!("[AssetLoader] Clip {} '{}' → store id {}, {:.2}s, {} channels",
                i, clip.name, id, clip.duration, clip.channels.len());
            id
        })
        .collect();
    let clip_names: Vec<String> = prepared
        .animation_clips
        .iter()
        .map(|c| c.name.clone())
        .collect();

    log::debug!("[AssetLoader] Registered {} skeleton(s), {} clip(s)", skeleton_ids.len(), clip_ids.len());

    let total_meshes = prepared.meshes.len() + prepared.skinned_meshes.len();

    // If multiple meshes, create a root parent entity so they move together
    let root_id = if total_meshes > 1 {
        let root = world.spawn_entity();
        let asset_name = std::path::Path::new(&prepared.asset_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Model");
        world.set_name(root, asset_name);
        world.set_transform(
            root,
            core::Transform {
                position: drop_pos,
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
        );
        Some(root)
    } else {
        None
    };

    let mut first_id = root_id;

    // Normal meshes
    for gltf_mesh in &prepared.meshes {
        let mesh_id = if let Some(id) = scene.mesh_store.find_by_name(&gltf_mesh.store_name) {
            id
        } else {
            scene.mesh_store.add_named(
                &gpu.device,
                &gltf_mesh.vertices,
                &gltf_mesh.indices,
                &gltf_mesh.store_name,
            )
        };

        let id = world.spawn_entity();
        world.set_name(id, &gltf_mesh.display_name);

        let (scale, rotation, translation) = gltf_mesh.transform.to_scale_rotation_translation();
        if let Some(root) = root_id {
            // Child: local transform relative to root (root is at drop_pos)
            world.set_transform(
                id,
                core::Transform { position: translation, rotation, scale },
            );
            world.set_parent(id, root);
        } else {
            // Single mesh: absolute position
            world.set_transform(
                id,
                core::Transform { position: translation + drop_pos, rotation, scale },
            );
        }

        let mat = build_material(&gltf_mesh.material, &prepared.textures, tex_ids);
        world.set_material(id, mat);

        world.set_mesh_renderer(
            id,
            core::MeshRenderer {
                mesh_id: Some(mesh_id),
                visible: true,
            },
        );

        if first_id.is_none() {
            first_id = Some(id);
        }
    }

    // Skinned meshes
    for sm in &prepared.skinned_meshes {
        let mesh_id = if let Some(id) = scene.mesh_store.find_by_name(&sm.store_name) {
            id
        } else {
            scene.mesh_store.add_skinned_named(
                &gpu.device,
                &sm.vertices,
                &sm.indices,
                &sm.store_name,
            )
        };

        let id = world.spawn_entity();
        world.set_name(id, &sm.display_name);

        let (scale, rotation, translation) = sm.transform.to_scale_rotation_translation();
        if let Some(root) = root_id {
            // Child: local transform relative to root
            world.set_transform(
                id,
                core::Transform { position: translation, rotation, scale },
            );
            world.set_parent(id, root);
        } else {
            // Single mesh: absolute position
            world.set_transform(
                id,
                core::Transform { position: translation + drop_pos, rotation, scale },
            );
        }

        let mat = build_material(&sm.material, &prepared.textures, tex_ids);
        world.set_material(id, mat);

        world.set_mesh_renderer(
            id,
            core::MeshRenderer {
                mesh_id: Some(mesh_id),
                visible: true,
            },
        );

        // Attach SkeletalAnimator with skeleton reference
        let skel_store_id = skeleton_ids.get(sm.skeleton_index).copied();
        let skel_name = skel_store_id.and_then(|sid| scene.skeleton_store.get_name(sid).map(|s| s.to_string()));
        let has_clips = !clip_ids.is_empty();
        log::info!("[AssetLoader] Attaching SkeletalAnimator to entity {:?} '{}':", id, sm.display_name);
        log::info!("[AssetLoader]   skeleton: {:?} (store_id={:?})", skel_name, skel_store_id);
        log::info!("[AssetLoader]   clips: {} ({:?}), auto_play={}", clip_ids.len(), clip_names, has_clips);
        if skel_store_id.is_none() {
            log::warn!("[AssetLoader]   WARNING: no skeleton mapped (skeleton_index={} but only {} available)",
                sm.skeleton_index, skeleton_ids.len());
        }
        world.set_skeletal_animator(
            id,
            SkeletalAnimator {
                skeleton_id: skel_store_id,
                skeleton_name: skel_name,
                clip_ids: clip_ids.clone(),
                clip_names: clip_names.clone(),
                active_clip: if clip_ids.is_empty() { None } else { Some(0) },
                active_clip_name: clip_names.first().cloned(),
                playing: has_clips,
                loop_animation: true,
                ..SkeletalAnimator::default()
            },
        );

        if first_id.is_none() {
            first_id = Some(id);
        }
    }

    // ---- Attach components to root so they're visible in Inspector ----
    if let Some(root) = root_id {
        // Material from the first available mesh (skinned preferred)
        let first_mat_src = prepared
            .skinned_meshes
            .first()
            .map(|sm| &sm.material)
            .or(prepared.meshes.first().map(|m| &m.material));
        if let Some(mat_src) = first_mat_src {
            let mat = build_material(mat_src, &prepared.textures, tex_ids);
            world.set_material(root, mat);
        }

        // MeshRenderer (mesh_id = None — children hold the actual meshes)
        world.set_mesh_renderer(
            root,
            core::MeshRenderer {
                mesh_id: None,
                visible: true,
            },
        );

        // SkeletalAnimator on root acts as the "controller" visible in Inspector.
        // Parent→children sync in skeletal_animation_system propagates state to children.
        if !prepared.skinned_meshes.is_empty() {
            let skel_store_id = skeleton_ids.first().copied();
            let skel_name = skel_store_id
                .and_then(|sid| scene.skeleton_store.get_name(sid).map(|s| s.to_string()));
            log::info!(
                "[AssetLoader] Attaching root SkeletalAnimator to {:?}: skeleton={:?}, clips={}",
                root, skel_name, clip_ids.len()
            );
            world.set_skeletal_animator(
                root,
                SkeletalAnimator {
                    skeleton_id: skel_store_id,
                    skeleton_name: skel_name,
                    clip_ids: clip_ids.clone(),
                    clip_names: clip_names.clone(),
                    active_clip: if clip_ids.is_empty() { None } else { Some(0) },
                    active_clip_name: clip_names.first().cloned(),
                    playing: !clip_ids.is_empty(),
                    loop_animation: true,
                    ..SkeletalAnimator::default()
                },
            );
        }
    }

    // Compute overall min Y from all imported meshes to offset above ground
    let mut global_min_y = f32::MAX;
    for gltf_mesh in &prepared.meshes {
        if let Some(mesh_id) = scene.mesh_store.find_by_name(&gltf_mesh.store_name) {
            if let Some(aabb) = scene.mesh_store.get_aabb(mesh_id) {
                let (scale, rot, trans) = gltf_mesh.transform.to_scale_rotation_translation();
                let model = glam::Mat4::from_scale_rotation_translation(scale, rot, trans);
                let (world_min, _) = aabb.transformed(model);
                global_min_y = global_min_y.min(world_min.y);
            }
        }
    }
    for sm in &prepared.skinned_meshes {
        if let Some(mesh_id) = scene.mesh_store.find_by_name(&sm.store_name) {
            if let Some(aabb) = scene.mesh_store.get_aabb(mesh_id) {
                let (scale, rot, trans) = sm.transform.to_scale_rotation_translation();
                let model = glam::Mat4::from_scale_rotation_translation(scale, rot, trans);
                let (world_min, _) = aabb.transformed(model);
                global_min_y = global_min_y.min(world_min.y);
            }
        }
    }

    // If the lowest point is below ground, push everything up
    if global_min_y < -0.001 && global_min_y != f32::MAX {
        let y_offset = -global_min_y;
        if let Some(root) = root_id {
            if let Some(t) = world.get_transform_mut(root) {
                t.position.y += y_offset;
            }
        } else if let Some(id) = first_id {
            if let Some(t) = world.get_transform_mut(id) {
                t.position.y += y_offset;
            }
        }
        drop_pos.y += y_offset;
    }

    if let Some(id) = first_id {
        ec.select(id);
        // Auto-expand in hierarchy
        ec.hierarchy_expanded.insert(id);
        // Auto-focus camera on the imported model
        ec.pending_camera_focus = Some(drop_pos + glam::Vec3::new(0.0, 0.8, 0.0));
    }
    log::info!(
        "Loaded asset: {} ({} meshes, {} skinned, {} skeletons, {} animations)",
        prepared.asset_path,
        prepared.meshes.len(),
        prepared.skinned_meshes.len(),
        prepared.skeletons.len(),
        prepared.animation_clips.len(),
    );
}

fn build_material(
    gmat: &Option<crate::assets::gltf_loader::GltfLoadedMaterial>,
    textures: &[PrecomputedTexture],
    tex_ids: &[usize],
) -> core::Material {
    let mut mat = core::Material::default();
    if let Some(ref gm) = gmat {
        mat.albedo = gm.albedo;
        mat.roughness = gm.roughness;
        mat.metallic = gm.metallic;
        mat.emission = gm.emission;

        if let Some(tex_idx) = gm.albedo_texture {
            if let Some(&tid) = tex_ids.get(tex_idx) {
                mat.texture_id = Some(tid);
                mat.texture_path = Some(textures[tex_idx].cache_key.clone());
            }
        }
        if let Some(tex_idx) = gm.normal_texture {
            if let Some(&tid) = tex_ids.get(tex_idx) {
                mat.normal_map_id = Some(tid);
                mat.normal_map_path = Some(textures[tex_idx].cache_key.clone());
            }
        }
    }
    mat
}
