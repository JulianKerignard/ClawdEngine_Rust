use std::sync::mpsc;

use crate::assets::gltf_loader::{GltfLoadedMesh, GltfLoadedTexture};
use crate::core::{self, World};
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

    std::thread::spawn(move || {
        let result = (|| -> Result<PreparedGltfAsset, String> {
            let gltf_scene =
                crate::assets::gltf_loader::load_gltf(&path).map_err(|e| e.to_string())?;

            let textures: Vec<PrecomputedTexture> = gltf_scene
                .textures
                .into_iter()
                .map(precompute_mipmaps)
                .collect();

            Ok(PreparedGltfAsset {
                meshes: gltf_scene.meshes,
                textures,
                asset_path: path,
            })
        })();

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
                ec.asset_load_state = Some(AssetLoadState::Finalizing { prepared, drop_pos });
            }
            Ok(Err(e)) => {
                log::error!("Background glTF load failed: {}", e);
                ec.loading_status = None;
                ec.save_feedback = Some((format!("Load failed: {}", e), 3.0));
            }
            Err(mpsc::TryRecvError::Empty) => {
                ec.asset_load_state = Some(AssetLoadState::Parsing { receiver, drop_pos });
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                log::error!("Background loader thread panicked");
                ec.loading_status = None;
            }
        },

        AssetLoadState::Finalizing { prepared, drop_pos } => {
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
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    let mut first_id = None;
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
        world.set_transform(
            id,
            core::Transform {
                position: translation + drop_pos,
                rotation,
                scale,
            },
        );

        let mut mat = core::Material::default();
        if let Some(ref gmat) = gltf_mesh.material {
            mat.albedo = gmat.albedo;
            mat.roughness = gmat.roughness;
            mat.metallic = gmat.metallic;
            mat.emission = gmat.emission;

            if let Some(tex_idx) = gmat.albedo_texture {
                if let Some(&tid) = tex_ids.get(tex_idx) {
                    mat.texture_id = Some(tid);
                    mat.texture_path =
                        Some(prepared.textures[tex_idx].cache_key.clone());
                }
            }
            if let Some(tex_idx) = gmat.normal_texture {
                if let Some(&tid) = tex_ids.get(tex_idx) {
                    mat.normal_map_id = Some(tid);
                    mat.normal_map_path =
                        Some(prepared.textures[tex_idx].cache_key.clone());
                }
            }
        }
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

    if let Some(id) = first_id {
        ec.select(id);
    }
    log::info!(
        "Loaded glTF asset: {} ({} meshes)",
        prepared.asset_path,
        prepared.meshes.len()
    );
}
