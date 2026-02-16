use crate::assets;
use crate::core::{EntityId, World};
use crate::editor::context::EditorContext;
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting::GameScript;

pub(crate) fn process_play_toggle(
    world: &mut World,
    ec: &mut EditorContext,
    scripts_started: &mut bool,
) {
    let Some(new_mode) = ec.pending_play_toggle.take() else {
        return;
    };
    if new_mode {
        ec.snapshot = Some(world.snapshot());
        crate::audio::start_play_mode_audio(world);
        ec.play_mode = true;
    } else {
        if let Some(snap) = ec.snapshot.take() {
            world.restore(snap);
        }
        ec.play_mode = false;
    }
    *scripts_started = false;
}

pub(crate) fn process_new_scene(
    world: &mut World,
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some(scene_name) = ec.pending_new_scene_name.take() else {
        return;
    };
    let existing: Vec<EntityId> = world.iter_entities().collect();
    for eid in existing {
        world.destroy_entity(eid);
    }
    scripts.clear();
    ec.deselect_all();
    ec.undo_stack = crate::editor::context::UndoStack::new();
    ec.scene_name = scene_name;
    ec.save_feedback = Some(("New scene created".into(), 2.0));
    log::info!("New scene created");
}

pub(crate) fn process_save_scene(
    world: &World,
    ec: &mut EditorContext,
    scene: &SceneRenderer,
    gpu: &GpuContext,
    scripts: &[(EntityId, Box<dyn GameScript>)],
) {
    let Some(scene_name) = ec.pending_save_scene.take() else {
        return;
    };
    let scenes_dir = if ec.current_project_path.is_some() { "scenes" } else { "assets/scenes" };
    let path = format!("{}/{}.ron", scenes_dir, scene_name);
    std::fs::create_dir_all(scenes_dir).ok();
    match assets::scene::save_scene(world, &scene.mesh_store, scripts, &path) {
        Ok(()) => {
            let preview_path = format!("{}/{}.png", scenes_dir, scene_name);
            if let Err(e) = scene
                .viewport
                .capture_to_png(&gpu.device, &gpu.queue, &preview_path)
            {
                log::warn!("Preview save failed: {}", e);
            }
            ec.save_feedback = Some(("Scene saved!".into(), 2.0));
            log::info!("Saved scene: {}", path);
        }
        Err(e) => {
            ec.save_feedback = Some((format!("Save failed: {}", e), 3.0));
            log::error!("Save failed: {}", e);
        }
    }
}

pub(crate) fn process_load_scene(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some(scene_name) = ec.pending_load_scene.take() else {
        return;
    };
    let scenes_dir = if ec.current_project_path.is_some() { "scenes" } else { "assets/scenes" };
    let path = format!("{}/{}.ron", scenes_dir, scene_name);
    if !std::path::Path::new(&path).exists() {
        ec.save_feedback = Some(("No scene file found".into(), 2.0));
        log::warn!("Scene file not found: {}", path);
        return;
    }
    match assets::scene::load_scene(
        world,
        &mut scene.mesh_store,
        scripts,
        &ec.script_registry,
        &gpu.device,
        &path,
    ) {
        Ok(()) => {
            let entity_ids: Vec<_> = world.iter_entities().collect();
            for eid in entity_ids {
                if let Some(mat) = world.get_material_mut(eid) {
                    if let Some(ref path) = mat.texture_path {
                        let tid = scene.texture_store.load(&gpu.device, &gpu.queue, path);
                        mat.texture_id = Some(tid);
                    }
                    if let Some(ref path) = mat.normal_map_path {
                        let nid = scene.texture_store.load(&gpu.device, &gpu.queue, path);
                        mat.normal_map_id = Some(nid);
                    }
                }
            }
            ec.deselect_all();
            ec.undo_stack = crate::editor::context::UndoStack::new();
            ec.save_feedback = Some(("Scene loaded!".into(), 2.0));
            ec.scene_name = scene_name;
            log::info!("Loaded scene: {}", path);
        }
        Err(e) => {
            ec.save_feedback = Some((format!("Load failed: {}", e), 3.0));
            log::error!("Load failed: {}", e);
        }
    }
}
