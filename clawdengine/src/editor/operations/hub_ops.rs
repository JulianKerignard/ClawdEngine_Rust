use crate::core::{EntityId, World};
use crate::editor::context::{AppScreen, EditorContext, HubAction};
use crate::scripting::GameScript;

pub(crate) fn process_hub_action(
    world: &mut World,
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some(action) = ec.pending_hub_action.take() else {
        return;
    };

    let existing: Vec<EntityId> = world.iter_entities().collect();
    for eid in existing {
        world.destroy_entity(eid);
    }
    scripts.clear();
    ec.deselect_all();
    ec.undo_stack = crate::editor::context::UndoStack::new();

    match action {
        HubAction::NewBlank => {
            let projects_dir = crate::assets::project::default_projects_dir();
            let _ = std::fs::create_dir_all(&projects_dir);
            let parent = projects_dir.to_string_lossy().to_string();

            let mut name = "Untitled".to_string();
            let mut i = 2;
            while std::path::Path::new(&format!("{}/{}", parent, name)).exists() {
                name = format!("Untitled {}", i);
                i += 1;
            }

            match crate::assets::project::create_project(&parent, &name) {
                Ok(path) => {
                    let mut registry = crate::assets::project::load_registry();
                    crate::assets::project::register_project(&mut registry, &path);
                    let _ = crate::assets::project::save_registry(&registry);
                    let _ = std::env::set_current_dir(&path);
                    ec.current_project_path = Some(path);
                    ec.scene_name = "Untitled".to_string();
                    ec.asset_current_dir = std::path::PathBuf::from(".");
                    ec.project_settings = crate::assets::settings::load_settings();
                    ec.screen = AppScreen::Editor;
                    ec.refresh_assets();
                    log::info!("Created new blank project: {}", name);
                }
                Err(e) => {
                    log::error!("Failed to create project: {}", e);
                }
            }
        }
        HubAction::NewDemo => {
            let projects_dir = crate::assets::project::default_projects_dir();
            let _ = std::fs::create_dir_all(&projects_dir);
            let parent = projects_dir.to_string_lossy().to_string();

            let mut name = "Demo Scene".to_string();
            let mut i = 2;
            while std::path::Path::new(&format!("{}/{}", parent, name)).exists() {
                name = format!("Demo Scene {}", i);
                i += 1;
            }

            match crate::assets::project::create_project(&parent, &name) {
                Ok(path) => {
                    let mut registry = crate::assets::project::load_registry();
                    crate::assets::project::register_project(&mut registry, &path);
                    let _ = crate::assets::project::save_registry(&registry);
                    let _ = std::env::set_current_dir(&path);

                    let orig = &ec.original_cwd;
                    copy_demo_assets(orig, &path);

                    *scripts = crate::editor::default_scene::setup_default_scene(
                        world,
                        ec.builtin_meshes.cube,
                        ec.builtin_meshes.sphere,
                    );
                    ec.current_project_path = Some(path);
                    ec.scene_name = "Demo Scene".to_string();
                    ec.asset_current_dir = std::path::PathBuf::from(".");
                    ec.project_settings = crate::assets::settings::load_settings();
                    ec.screen = AppScreen::Editor;
                    ec.refresh_assets();
                    log::info!("Created demo project: {}", name);
                }
                Err(e) => {
                    log::error!("Failed to create demo project: {}", e);
                }
            }
        }
        HubAction::OpenProject(path) => {
            let mut registry = crate::assets::project::load_registry();
            crate::assets::project::register_project(&mut registry, &path);
            let _ = crate::assets::project::save_registry(&registry);

            let _ = std::env::set_current_dir(&path);
            ec.current_project_path = Some(path.clone());

            if let Ok(manifest) = crate::assets::project::load_manifest(&path) {
                ec.scene_name = manifest.name.clone();
                if let Some(scene_name) = &manifest.default_scene {
                    ec.pending_load_scene = Some(scene_name.clone());
                }
            } else {
                ec.scene_name = "Unknown".to_string();
            }

            ec.asset_current_dir = std::path::PathBuf::from(".");
            ec.project_settings = crate::assets::settings::load_settings();
            ec.screen = AppScreen::Editor;
            ec.refresh_assets();
            log::info!("Opened project: {}", path);
        }
    }
}

fn copy_demo_assets(original_cwd: &str, project_path: &str) {
    let src_base = if original_cwd.is_empty() {
        crate::assets::paths::resolve("assets")
    } else {
        std::path::Path::new(original_cwd).join("assets")
    };
    let dst_base = std::path::Path::new(project_path);

    if let Ok(entries) = std::fs::read_dir(src_base.join("textures")) {
        for entry in entries.flatten() {
            let src = entry.path();
            if src.is_file() {
                let dst = dst_base.join("textures").join(entry.file_name());
                let _ = std::fs::copy(&src, &dst);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(src_base.join("audio")) {
        for entry in entries.flatten() {
            let src = entry.path();
            if src.is_file() {
                let dst = dst_base.join("audio").join(entry.file_name());
                let _ = std::fs::copy(&src, &dst);
            }
        }
    }
}
