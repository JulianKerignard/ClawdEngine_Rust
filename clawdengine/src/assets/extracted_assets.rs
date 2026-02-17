use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::gltf_loader::GltfLoadedMaterial;
use crate::core::{AnimationClip, Skeleton};
use crate::editor::asset_loader::PreparedGltfAsset;

/// Material data extracted from an imported model, serializable to .mat.ron
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractedMaterial {
    pub name: String,
    pub albedo: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub emission: [f32; 3],
    pub albedo_texture: Option<String>, // relative path to extracted texture
    pub normal_texture: Option<String>,
}

/// Convenience wrapper: extract sub-assets from a `PreparedGltfAsset`.
///
/// Collects materials and texture data from the prepared asset and delegates
/// to [`extract_subassets`].
pub fn extract_subassets_from_prepared(prepared: &PreparedGltfAsset) -> Result<PathBuf, String> {
    // Collect (mesh_display_name, material_ref) from both mesh types
    let meshes_materials: Vec<(String, &Option<GltfLoadedMaterial>)> = prepared
        .meshes
        .iter()
        .map(|m| (m.display_name.clone(), &m.material))
        .chain(
            prepared
                .skinned_meshes
                .iter()
                .map(|m| (m.display_name.clone(), &m.material)),
        )
        .collect();

    // Collect texture data: (cache_key, rgba_level0, width, height)
    let textures: Vec<(String, &[u8], u32, u32)> = prepared
        .textures
        .iter()
        .filter_map(|t| {
            t.mip_levels
                .first()
                .map(|rgba| (t.cache_key.clone(), rgba.as_slice(), t.width, t.height))
        })
        .collect();

    extract_subassets(
        &prepared.asset_path,
        &prepared.skeletons,
        &prepared.animation_clips,
        &meshes_materials,
        &textures,
    )
}

/// Extract sub-assets from a parsed model as flat files next to the source.
///
/// Given `asset_path` `"OldManIdle.fbx"`, creates files at the same level:
/// - `OldManIdle_Armature.skel.ron`
/// - `OldManIdle_Idle.clip.ron`
/// - `OldManIdle_Material_0.mat.ron`
/// - `OldManIdle_texture_0.png`
///
/// No folders are created.
pub fn extract_subassets(
    asset_path: &str,
    skeletons: &[Skeleton],
    clips: &[AnimationClip],
    meshes_materials: &[(String, &Option<GltfLoadedMaterial>)],
    textures: &[(String, &[u8], u32, u32)], // (cache_key, rgba, w, h)
) -> Result<PathBuf, String> {
    let src = Path::new(asset_path);
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
    let prefix = sanitize_name(stem);
    let out_dir = src.parent().unwrap_or(Path::new("."));

    // Skip if first sub-asset already exists (idempotent)
    let marker = out_dir.join(format!("{}_.extracted", prefix));
    if marker.exists() {
        log::info!("[Extract] '{}' already extracted, skipping", asset_path);
        return Ok(out_dir.to_path_buf());
    }

    log::info!(
        "[Extract] Extracting sub-assets from '{}' (prefix='{}') into '{}'",
        asset_path, prefix, out_dir.display()
    );

    // --- Skeletons ---
    for (i, skeleton) in skeletons.iter().enumerate() {
        let skel_name = sanitize_name(&skeleton.name);
        let filename = if skeletons.len() == 1 {
            format!("{}_{}.skel.ron", prefix, skel_name)
        } else {
            format!("{}_{}_{}.skel.ron", prefix, skel_name, i)
        };
        let path = out_dir.join(&filename);
        let ron = ron::ser::to_string_pretty(skeleton, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize skeleton: {}", e))?;
        std::fs::write(&path, &ron)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
        log::info!("[Extract]   {}", filename);
    }

    // --- Animation Clips ---
    for (i, clip) in clips.iter().enumerate() {
        let clip_name = sanitize_name(&clip.name);
        let filename = if clips.len() == 1 {
            format!("{}_{}.clip.ron", prefix, clip_name)
        } else {
            format!("{}_{}_{}.clip.ron", prefix, clip_name, i)
        };
        let path = out_dir.join(&filename);
        let ron = ron::ser::to_string_pretty(clip, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize clip: {}", e))?;
        std::fs::write(&path, &ron)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
        log::info!("[Extract]   {} ({:.2}s, {} ch)", filename, clip.duration, clip.channels.len());
    }

    // --- Textures (embedded) ---
    let mut texture_filenames: Vec<String> = Vec::new();
    for (i, (cache_key, rgba, width, height)) in textures.iter().enumerate() {
        let filename = format!("{}_texture_{}.png", prefix, i);
        let path = out_dir.join(&filename);
        if let Err(e) = write_png(&path, rgba, *width, *height) {
            log::warn!("[Extract]   Failed to write {} (key='{}'): {}", filename, cache_key, e);
            texture_filenames.push(String::new());
            continue;
        }
        log::info!("[Extract]   {} ({}x{})", filename, width, height);
        texture_filenames.push(filename);
    }

    // --- Materials ---
    let mut materials_seen: Vec<String> = Vec::new();
    for (idx, (mesh_name, mat_opt)) in meshes_materials.iter().enumerate() {
        if let Some(mat) = mat_opt {
            let mat_name = format!("Material_{}", idx);
            if materials_seen.contains(&mat_name) {
                continue;
            }
            materials_seen.push(mat_name.clone());

            let extracted = ExtractedMaterial {
                name: mat_name.clone(),
                albedo: [mat.albedo.x, mat.albedo.y, mat.albedo.z],
                roughness: mat.roughness,
                metallic: mat.metallic,
                emission: [mat.emission.x, mat.emission.y, mat.emission.z],
                albedo_texture: mat
                    .albedo_texture
                    .and_then(|ti| texture_filenames.get(ti))
                    .filter(|s| !s.is_empty())
                    .cloned(),
                normal_texture: mat
                    .normal_texture
                    .and_then(|ti| texture_filenames.get(ti))
                    .filter(|s| !s.is_empty())
                    .cloned(),
            };

            let filename = format!("{}_{}.mat.ron", prefix, sanitize_name(&mat_name));
            let path = out_dir.join(&filename);
            let ron = ron::ser::to_string_pretty(&extracted, ron::ser::PrettyConfig::default())
                .map_err(|e| format!("Failed to serialize material: {}", e))?;
            std::fs::write(&path, &ron)
                .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
            log::info!("[Extract]   {} (from mesh '{}')", filename, mesh_name);
        }
    }

    // Write marker so we don't re-extract next time
    let _ = std::fs::write(&marker, "");

    let total = skeletons.len() + clips.len() + textures.len() + materials_seen.len();
    log::info!("[Extract] Extracted {} sub-assets for '{}'", total, prefix);

    Ok(out_dir.to_path_buf())
}

/// Write RGBA pixel data as a PNG file using the `image` crate.
fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), String> {
    let img: image::ImageBuffer<image::Rgba<u8>, _> =
        image::ImageBuffer::from_raw(width, height, rgba.to_vec())
            .ok_or_else(|| "RGBA buffer size mismatch".to_string())?;
    img.save(path)
        .map_err(|e| format!("Failed to save PNG: {}", e))
}

/// Sanitize a name for use as a filename.
fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}
