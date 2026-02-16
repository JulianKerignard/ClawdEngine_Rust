use crate::assets;
use crate::core::{self, World};
use crate::editor::context::{EditorContext, ScriptRegistryEntry};
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting;

fn is_valid_asset_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 255 {
        return false;
    }
    if name.contains("..") || name.starts_with('.') {
        return false;
    }
    name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' || c == '.')
}

fn compute_drop_position(
    ec: &mut EditorContext,
    camera: &crate::renderer::Camera,
) -> glam::Vec3 {
    let Some((sx, sy)) = ec.pending_drop_screen_pos.take() else {
        return glam::Vec3::ZERO;
    };
    let vp = ec.viewport_rect;
    let vp_w = vp.width();
    let vp_h = vp.height();
    if vp_w < 1.0 || vp_h < 1.0 {
        return glam::Vec3::ZERO;
    }

    let (ray_origin, ray_dir) = camera.screen_to_ray(sx, sy, vp_w, vp_h);

    if ray_dir.y.abs() > 1e-6 {
        let t = -ray_origin.y / ray_dir.y;
        if t > 0.0 && t < 200.0 {
            return ray_origin + ray_dir * t;
        }
    }
    ray_origin + ray_dir * 5.0
}

pub(crate) fn process_load_asset(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
) {
    let Some(asset_path) = ec.pending_load_asset.take() else {
        return;
    };

    let drop_pos = compute_drop_position(ec, &scene.camera);

    if asset_path.ends_with(".glb") || asset_path.ends_with(".gltf") || asset_path.ends_with(".fbx") {
        if ec.asset_load_state.is_some() {
            ec.save_feedback = Some(("Another asset is loading...".into(), 2.0));
            return;
        }
        let rx = crate::editor::asset_loader::start_background_load(&asset_path);
        ec.loading_status = Some("Parsing 3D model...".into());
        ec.asset_load_state = Some(crate::editor::asset_loader::AssetLoadState::Parsing {
            receiver: rx,
            drop_pos,
        });
    } else {
        load_obj_asset(world, ec, scene, gpu, &asset_path, drop_pos);
    }
}

fn load_obj_asset(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
    asset_path: &str,
    drop_pos: glam::Vec3,
) {
    match assets::obj_loader::load_obj(asset_path) {
        Ok(loaded) => {
            if let Some(lm) = loaded.into_iter().next() {
                let mesh_id =
                    scene
                        .mesh_store
                        .add_named(&gpu.device, &lm.vertices, &lm.indices, asset_path);
                ec.undo_stack
                    .push(world.snapshot(), ec.selected_entities.clone());
                let id = world.spawn_entity();
                let display_name = std::path::Path::new(asset_path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Asset".to_string());
                world.set_name(id, &display_name);
                world.set_transform(id, core::Transform {
                    position: drop_pos,
                    ..Default::default()
                });
                world.set_material(id, core::Material::default());
                world.set_mesh_renderer(
                    id,
                    core::MeshRenderer {
                        mesh_id: Some(mesh_id),
                        visible: true,
                    },
                );
                ec.select(id);
                log::info!("Loaded OBJ asset: {}", asset_path);
            }
        }
        Err(e) => {
            log::error!("Failed to load {}: {}", asset_path, e);
        }
    }
}

pub(crate) fn process_delete_asset(ec: &mut EditorContext) {
    let Some(path) = ec.pending_delete_asset.take() else {
        return;
    };
    let result = if path.is_dir() {
        std::fs::remove_dir_all(&path)
    } else {
        std::fs::remove_file(&path)
    };
    match result {
        Ok(()) => {
            log::info!("Deleted: {}", path.display());
            if path.extension().is_some_and(|e| e == "rs") {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    ec.script_registry.retain(|e| e.name != name);
                }
            }
            ec.refresh_assets();
        }
        Err(e) => {
            log::error!("Failed to delete {}: {}", path.display(), e);
        }
    }
}

pub(crate) fn process_create_folder(ec: &mut EditorContext) {
    let Some(folder_name) = ec.pending_create_folder.take() else {
        return;
    };
    if !is_valid_asset_name(&folder_name) {
        log::error!("Invalid folder name: '{}'", folder_name);
        return;
    }
    let folder_path = ec.asset_current_dir.join(&folder_name);
    if let Err(e) = std::fs::create_dir_all(&folder_path) {
        log::error!(
            "Failed to create folder {}: {}",
            folder_path.display(),
            e
        );
    } else {
        log::info!("Created folder: {}", folder_path.display());
        ec.refresh_assets();
    }
}

pub(crate) fn process_create_script(ec: &mut EditorContext) {
    let Some(script_name) = ec.pending_create_script.take() else {
        return;
    };
    let base_name = script_name.trim_end_matches(".rs");
    if !is_valid_asset_name(base_name) {
        log::error!("Invalid script name: '{}'", script_name);
        return;
    }
    let filename = if script_name.ends_with(".rs") {
        script_name
    } else {
        format!("{}.rs", script_name)
    };
    let script_path = ec.asset_current_dir.join(&filename);
    let struct_name: String = filename
        .trim_end_matches(".rs")
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect();
    let template = format!(
        "use crate::scripting::{{GameScript, ScriptContext}};\n\n\
         pub struct {} {{\n    // Add fields here\n}}\n\n\
         impl {} {{\n    pub fn new() -> Self {{\n        Self {{}}\n    }}\n}}\n\n\
         impl GameScript for {} {{\n    \
         fn name(&self) -> &str {{\n        \"{}\"\n    }}\n\n    \
         fn start(&mut self, _ctx: &mut ScriptContext) {{\n        \
         // Called once when play mode starts\n    }}\n\n    \
         fn update(&mut self, _ctx: &mut ScriptContext, _dt: f32) {{\n        \
         // Called every frame\n    }}\n}}\n",
        struct_name, struct_name, struct_name, struct_name
    );
    if let Err(e) = std::fs::write(&script_path, template) {
        log::error!(
            "Failed to create script {}: {}",
            script_path.display(),
            e
        );
    } else {
        log::info!("Created script: {}", script_path.display());
        let script_display = filename.trim_end_matches(".rs").to_string();
        let n = script_display.clone();
        ec.script_registry.push(ScriptRegistryEntry {
            name: script_display,
            factory: Box::new(move || Box::new(scripting::UserScript::new(n.clone()))),
        });
        ec.refresh_assets();
    }
}
