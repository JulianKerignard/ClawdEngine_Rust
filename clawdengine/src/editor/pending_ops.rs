use crate::core::{EntityId, World};
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting::GameScript;

use super::context::EditorContext;
use super::operations::{asset_ops, build_ops, component_ops, entity_ops, hub_ops, scene_ops};

/// Process all pending editor operations queued during the UI frame.
///
/// This function drains every `pending_*` field on `EditorContext` and applies
/// the corresponding mutations to `World`, `SceneRenderer`, and the script list.
pub fn process_pending_operations(
    world: &mut World,
    editor_ctx: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
    scripts_started: &mut bool,
    dt: f32,
) {
    hub_ops::process_hub_action(world, editor_ctx, scripts);
    component_ops::process_undo(world, editor_ctx);
    component_ops::process_redo(world, editor_ctx);
    entity_ops::process_spawn(world, editor_ctx);
    entity_ops::process_reparent(world, editor_ctx);
    entity_ops::process_delete(world, editor_ctx, scripts);
    entity_ops::process_duplicate(world, editor_ctx, scripts);
    component_ops::process_remove_scripts(editor_ctx, scripts);
    component_ops::process_add_script(editor_ctx, scripts);
    component_ops::process_texture_assign(world, editor_ctx, scene, gpu);
    component_ops::process_normal_map_assign(world, editor_ctx, scene, gpu);
    component_ops::process_add_component(world, editor_ctx);
    asset_ops::process_load_asset(world, editor_ctx, scene, gpu);
    super::asset_loader::tick_loading(world, editor_ctx, scene, gpu);
    asset_ops::process_delete_asset(editor_ctx);
    asset_ops::process_create_folder(editor_ctx);
    asset_ops::process_create_script(editor_ctx);
    scene_ops::process_play_toggle(world, editor_ctx, scripts_started);
    scene_ops::process_new_scene(world, editor_ctx, scripts);
    scene_ops::process_save_scene(world, editor_ctx, scene, gpu, scripts);
    build_ops::process_build_game(editor_ctx);
    scene_ops::process_load_scene(world, editor_ctx, scene, gpu, scripts);
    tick_save_feedback(editor_ctx, dt);
    tick_settings_autosave(editor_ctx);
}

fn tick_save_feedback(ec: &mut EditorContext, dt: f32) {
    if let Some((_, ref mut time)) = ec.save_feedback {
        *time -= dt;
        if *time <= 0.0 {
            ec.save_feedback = None;
        }
    }
}

fn tick_settings_autosave(ec: &mut EditorContext) {
    if ec.settings_dirty && ec.current_project_path.is_some() {
        let _ = crate::assets::settings::save_settings(&ec.project_settings);
        ec.settings_dirty = false;
    }
}
