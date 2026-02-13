use crate::assets;
use crate::core::{self, EntityId, World};
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting::{self, GameScript};

use super::context::{
    AppScreen, ComponentKind, EditorContext, HubAction, ScriptRegistryEntry, SpawnRequest,
};

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
    process_hub_action(world, editor_ctx, scripts);
    process_undo(world, editor_ctx);
    process_redo(world, editor_ctx);
    process_spawn(world, editor_ctx);
    process_reparent(world, editor_ctx);
    process_delete(world, editor_ctx);
    process_duplicate(world, editor_ctx, scripts);
    process_remove_scripts(editor_ctx, scripts);
    process_add_script(editor_ctx, scripts);
    process_texture_assign(world, editor_ctx, scene, gpu);
    process_normal_map_assign(world, editor_ctx, scene, gpu);
    process_add_component(world, editor_ctx);
    process_load_asset(world, editor_ctx, scene, gpu);
    super::asset_loader::tick_loading(world, editor_ctx, scene, gpu);
    process_delete_asset(editor_ctx);
    process_create_folder(editor_ctx);
    process_create_script(editor_ctx);
    process_play_toggle(world, editor_ctx, scripts_started);
    process_new_scene(world, editor_ctx, scripts);
    process_save_scene(world, editor_ctx, scene, gpu, scripts);
    process_load_scene(world, editor_ctx, scene, gpu, scripts);
    tick_save_feedback(editor_ctx, dt);
}

fn process_hub_action(
    world: &mut World,
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some(action) = ec.pending_hub_action.take() else {
        return;
    };

    // Clean existing world (handles returning from editor to hub then selecting again)
    let existing: Vec<EntityId> = world.iter_entities().collect();
    for eid in existing {
        world.destroy_entity(eid);
    }
    scripts.clear();
    ec.deselect_all();
    ec.undo_stack = super::context::UndoStack::new();

    match action {
        HubAction::NewBlank => {
            ec.scene_name = "Untitled".to_string();
            ec.screen = AppScreen::Editor;
            log::info!("New blank scene");
        }
        HubAction::NewDemo => {
            *scripts = super::default_scene::setup_default_scene(
                world,
                ec.builtin_meshes.cube,
                ec.builtin_meshes.sphere,
            );
            ec.scene_name = "Demo Scene".to_string();
            ec.screen = AppScreen::Editor;
            log::info!("Loaded demo scene");
        }
        HubAction::OpenScene(name) => {
            ec.pending_load_scene = Some(name);
            ec.screen = AppScreen::Editor;
        }
    }
}

fn process_undo(world: &mut World, ec: &mut EditorContext) {
    if !ec.pending_undo {
        return;
    }
    ec.pending_undo = false;
    if let Some(entry) = ec.undo_stack.pop() {
        ec.undo_stack.push_redo(world.snapshot(), ec.selected_entities.clone());
        world.restore(entry.snapshot);
        ec.selected_entities = entry.selected;
    }
}

fn process_redo(world: &mut World, ec: &mut EditorContext) {
    if !ec.pending_redo {
        return;
    }
    ec.pending_redo = false;
    if let Some(entry) = ec.undo_stack.pop_redo() {
        ec.undo_stack.push_undo_only(world.snapshot(), ec.selected_entities.clone());
        world.restore(entry.snapshot);
        ec.selected_entities = entry.selected;
    }
}

fn process_spawn(world: &mut World, ec: &mut EditorContext) {
    let Some(spawn) = ec.pending_spawn.take() else {
        return;
    };
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    let cube_mesh = ec.builtin_meshes.cube;
    let sphere_mesh = ec.builtin_meshes.sphere;

    let id = match spawn {
        SpawnRequest::Empty => {
            let id = world.spawn_entity();
            world.set_name(id, "Empty");
            world.set_transform(id, core::Transform::default());
            id
        }
        SpawnRequest::Cube => {
            let id = world.spawn_entity();
            world.set_name(id, "Cube");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(
                id,
                core::MeshRenderer {
                    mesh_id: Some(cube_mesh),
                    visible: true,
                },
            );
            id
        }
        SpawnRequest::Sphere => {
            let id = world.spawn_entity();
            world.set_name(id, "Sphere");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(
                id,
                core::MeshRenderer {
                    mesh_id: Some(sphere_mesh),
                    visible: true,
                },
            );
            id
        }
        SpawnRequest::Light => {
            let id = world.spawn_entity();
            world.set_name(id, "Light");
            world.set_transform(
                id,
                core::Transform {
                    position: glam::Vec3::new(0.0, 3.0, 0.0),
                    ..Default::default()
                },
            );
            world.set_light(id, core::Light::default());
            id
        }
        SpawnRequest::Camera => {
            let id = world.spawn_entity();
            world.set_name(id, "Camera");
            world.set_transform(id, core::Transform {
                position: glam::Vec3::new(0.0, 2.0, 5.0),
                rotation: glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    -15_f32.to_radians(),
                    0.0,
                    0.0,
                ),
                ..Default::default()
            });
            world.set_camera(id, core::CameraComponent::default());
            id
        }
        SpawnRequest::Audio => {
            let id = world.spawn_entity();
            world.set_name(id, "Audio Source");
            world.set_transform(id, core::Transform::default());
            world.set_audio_source(id, core::AudioSource::default());
            id
        }
        SpawnRequest::Canvas => {
            let id = world.spawn_entity();
            world.set_name(id, "Canvas");
            world.set_canvas(id, core::Canvas::default());
            id
        }
        SpawnRequest::UiText => {
            let canvas_eid: Option<_> = world.iter_entities().find(|&e| world.get_canvas(e).is_some());
            let id = world.spawn_entity();
            world.set_name(id, "Text");
            world.set_ui_element(id, core::UiElement::default());
            if let Some(cv) = canvas_eid { world.set_parent(id, cv); }
            id
        }
        SpawnRequest::UiPanel => {
            let canvas_eid: Option<_> = world.iter_entities().find(|&e| world.get_canvas(e).is_some());
            let id = world.spawn_entity();
            world.set_name(id, "Panel");
            world.set_ui_element(id, core::UiElement {
                kind: core::UiElementKind::Panel,
                text: String::new(),
                size: [200.0, 100.0],
                color: glam::Vec3::new(0.2, 0.2, 0.3),
                alpha: 0.8,
                ..Default::default()
            });
            if let Some(cv) = canvas_eid { world.set_parent(id, cv); }
            id
        }
    };
    ec.select(id);
}

fn collect_descendants(world: &World, id: EntityId, out: &mut Vec<EntityId>) {
    if out.contains(&id) { return; }
    out.push(id);
    for &child in world.get_children(id) {
        collect_descendants(world, child, out);
    }
}

fn process_delete(world: &mut World, ec: &mut EditorContext) {
    if ec.pending_delete.is_empty() {
        return;
    }
    let to_delete = std::mem::take(&mut ec.pending_delete);
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    // Collect descendants recursively
    let mut all = Vec::new();
    for id in &to_delete {
        collect_descendants(world, *id, &mut all);
    }
    // Delete children first (reverse order)
    all.reverse();
    for id in all {
        world.destroy_entity(id);
    }
    ec.deselect_all();
}

fn process_reparent(world: &mut World, ec: &mut EditorContext) {
    let Some((child, new_parent)) = ec.pending_reparent.take() else { return; };
    ec.undo_stack.push(world.snapshot(), ec.selected_entities.clone());
    match new_parent {
        Some(parent) => world.set_parent(child, parent),
        None => world.remove_parent(child),
    }
}

fn process_duplicate(
    world: &mut World,
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    if ec.pending_duplicate.is_empty() {
        return;
    }
    let to_duplicate = std::mem::take(&mut ec.pending_duplicate);
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    let mut new_ids = Vec::new();
    let mut id_map = std::collections::HashMap::new();
    for src_id in &to_duplicate {
        if !world.is_alive(*src_id) {
            continue;
        }
        let new_id = world.spawn_entity();

        if let Some(name) = world.get_name(*src_id) {
            let new_name = format!("{} (Copy)", name);
            world.set_name(new_id, &new_name);
        }
        if let Some(t) = world.get_transform(*src_id) {
            let mut new_t = *t;
            new_t.position += glam::Vec3::new(1.0, 0.0, 0.0);
            world.set_transform(new_id, new_t);
        }
        if let Some(mat) = world.get_material(*src_id) {
            world.set_material(new_id, mat.clone());
        }
        if let Some(mr) = world.get_mesh_renderer(*src_id) {
            world.set_mesh_renderer(new_id, mr.clone());
        }
        if let Some(light) = world.get_light(*src_id) {
            world.set_light(new_id, *light);
        }
        if let Some(rb) = world.get_rigid_body(*src_id) {
            world.set_rigid_body(new_id, *rb);
        }
        if let Some(col) = world.get_collider(*src_id) {
            world.set_collider(new_id, *col);
        }
        if let Some(cam) = world.get_camera(*src_id) {
            world.set_camera(new_id, *cam);
        }
        if let Some(audio) = world.get_audio_source(*src_id) {
            world.set_audio_source(new_id, audio.clone());
        }
        if let Some(al) = world.get_audio_listener(*src_id) {
            world.set_audio_listener(new_id, *al);
        }
        if let Some(ui) = world.get_ui_element(*src_id) {
            world.set_ui_element(new_id, ui.clone());
        }
        if let Some(cv) = world.get_canvas(*src_id) {
            world.set_canvas(new_id, *cv);
        }

        id_map.insert(*src_id, new_id);

        // Clone scripts via registry factory
        let script_names: Vec<String> = scripts
            .iter()
            .filter(|(id, _)| *id == *src_id)
            .map(|(_, s)| s.name().to_string())
            .collect();
        for sname in &script_names {
            if let Some(entry) = ec.script_registry.iter().find(|e| e.name == *sname) {
                let new_script = (entry.factory)();
                scripts.push((new_id, new_script));
            }
        }

        new_ids.push(new_id);
    }

    // Restore parent hierarchy (if parent was also duplicated)
    for src_id in &to_duplicate {
        if let Some(parent_id) = world.get_parent(*src_id) {
            if let Some(&new_parent) = id_map.get(&parent_id) {
                if let Some(&new_child) = id_map.get(src_id) {
                    world.set_parent(new_child, new_parent);
                }
            }
        }
    }

    ec.selected_entities = new_ids;
}

fn process_remove_scripts(
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    if ec.pending_remove_scripts.is_empty() {
        return;
    }
    let removals = std::mem::take(&mut ec.pending_remove_scripts);
    for (eid, sname) in removals {
        if let Some(pos) = scripts
            .iter()
            .position(|(id, s)| *id == eid && s.name() == sname)
        {
            scripts.remove(pos);
        }
    }
}

fn process_add_script(
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some((eid, idx)) = ec.pending_add_script.take() else {
        return;
    };
    if idx < ec.script_registry.len() {
        let script = (ec.script_registry[idx].factory)();
        scripts.push((eid, script));
    }
}

fn process_texture_assign(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
) {
    let Some((eid, tex_path)) = ec.pending_texture_assign.take() else {
        return;
    };
    let tex_id = scene.texture_store.load(&gpu.device, &gpu.queue, &tex_path);
    if let Some(mat) = world.get_material_mut(eid) {
        mat.texture_path = Some(tex_path);
        mat.texture_id = Some(tex_id);
    }
}

fn process_normal_map_assign(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
) {
    let Some((eid, nmap_path)) = ec.pending_normal_map_assign.take() else {
        return;
    };
    let nmap_id = scene
        .texture_store
        .load(&gpu.device, &gpu.queue, &nmap_path);
    if let Some(mat) = world.get_material_mut(eid) {
        mat.normal_map_path = Some(nmap_path);
        mat.normal_map_id = Some(nmap_id);
    }
}

fn process_add_component(world: &mut World, ec: &mut EditorContext) {
    let Some((eid, kind)) = ec.pending_add_component.take() else {
        return;
    };
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());
    match kind {
        ComponentKind::Material => {
            world.set_material(eid, core::Material::default());
        }
        ComponentKind::MeshRenderer => {
            world.set_mesh_renderer(eid, core::MeshRenderer::default());
        }
        ComponentKind::RigidBody => {
            world.set_rigid_body(eid, core::RigidBody::default());
        }
        ComponentKind::Collider => {
            world.set_collider(eid, core::Collider::default());
        }
        ComponentKind::CameraComponent => {
            world.set_camera(eid, core::CameraComponent::default());
        }
        ComponentKind::AudioSource => {
            world.set_audio_source(eid, core::AudioSource::default());
        }
        ComponentKind::AudioListener => {
            world.set_audio_listener(eid, core::AudioListener::default());
        }
        ComponentKind::UiElement => {
            world.set_ui_element(eid, core::UiElement::default());
        }
        ComponentKind::Canvas => {
            world.set_canvas(eid, core::Canvas::default());
        }
    }
}

/// Compute world-space drop position from viewport-relative screen coords.
/// Casts a ray through the cursor and intersects with the Y=0 ground plane.
/// Falls back to a point 5 units in front of the camera if the ray is parallel.
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

    // Intersect with Y=0 ground plane
    if ray_dir.y.abs() > 1e-6 {
        let t = -ray_origin.y / ray_dir.y;
        if t > 0.0 && t < 200.0 {
            return ray_origin + ray_dir * t;
        }
    }
    // Fallback: place 5 units in front of the camera
    ray_origin + ray_dir * 5.0
}

fn process_load_asset(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
) {
    let Some(asset_path) = ec.pending_load_asset.take() else {
        return;
    };

    let drop_pos = compute_drop_position(ec, &scene.camera);

    if asset_path.ends_with(".glb") || asset_path.ends_with(".gltf") {
        if ec.asset_load_state.is_some() {
            ec.save_feedback = Some(("Another asset is loading...".into(), 2.0));
            return;
        }
        let rx = super::asset_loader::start_background_load(&asset_path);
        ec.loading_status = Some("Parsing glTF...".into());
        ec.asset_load_state = Some(super::asset_loader::AssetLoadState::Parsing {
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

fn process_delete_asset(ec: &mut EditorContext) {
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

fn process_create_folder(ec: &mut EditorContext) {
    let Some(folder_name) = ec.pending_create_folder.take() else {
        return;
    };
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

fn process_create_script(ec: &mut EditorContext) {
    let Some(script_name) = ec.pending_create_script.take() else {
        return;
    };
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

fn process_play_toggle(
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

fn process_new_scene(
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
    ec.undo_stack = super::context::UndoStack::new();
    ec.scene_name = scene_name;
    ec.save_feedback = Some(("New scene created".into(), 2.0));
    log::info!("New scene created");
}

fn process_save_scene(
    world: &World,
    ec: &mut EditorContext,
    scene: &SceneRenderer,
    gpu: &GpuContext,
    scripts: &[(EntityId, Box<dyn GameScript>)],
) {
    let Some(scene_name) = ec.pending_save_scene.take() else {
        return;
    };
    let path = format!("assets/scenes/{}.ron", scene_name);
    std::fs::create_dir_all("assets/scenes").ok();
    match assets::scene::save_scene(world, &scene.mesh_store, scripts, &path) {
        Ok(()) => {
            let preview_path = format!("assets/scenes/{}.png", scene_name);
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

fn process_load_scene(
    world: &mut World,
    ec: &mut EditorContext,
    scene: &mut SceneRenderer,
    gpu: &GpuContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    let Some(scene_name) = ec.pending_load_scene.take() else {
        return;
    };
    let path = format!("assets/scenes/{}.ron", scene_name);
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
            // Resolve texture_path/normal_map_path -> IDs for all materials
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

fn tick_save_feedback(ec: &mut EditorContext, dt: f32) {
    if let Some((_, ref mut time)) = ec.save_feedback {
        *time -= dt;
        if *time <= 0.0 {
            ec.save_feedback = None;
        }
    }
}
