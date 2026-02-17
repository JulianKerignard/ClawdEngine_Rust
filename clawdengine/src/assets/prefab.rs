use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::core::{EntityId, World, MeshRenderer};
use crate::editor::context::ScriptRegistryEntry;
use crate::renderer::mesh::MeshStore;
use crate::scripting::GameScript;

use super::scene::{EntityData, MeshRef, ScriptData};

/// A prefab is a saved entity subtree (root + all descendants).
#[derive(Serialize, Deserialize)]
pub struct PrefabData {
    pub entities: Vec<EntityData>,
}

/// Collect an entity and all its descendants depth-first.
fn collect_subtree(world: &World, root: EntityId) -> Vec<EntityId> {
    let mut result = Vec::new();
    fn recurse(world: &World, eid: EntityId, out: &mut Vec<EntityId>) {
        out.push(eid);
        for &child in world.get_children(eid) {
            recurse(world, child, out);
        }
    }
    recurse(world, root, &mut result);
    result
}

/// Save an entity subtree as a `.prefab.ron` file.
pub fn save_prefab(
    world: &World,
    mesh_store: &MeshStore,
    scripts: &[(EntityId, Box<dyn GameScript>)],
    root: EntityId,
    path: &str,
) -> Result<()> {
    let subtree = collect_subtree(world, root);

    // Build EntityId → index mapping within the subtree
    let id_to_idx: std::collections::HashMap<EntityId, usize> = subtree.iter()
        .enumerate()
        .map(|(i, &eid)| (eid, i))
        .collect();

    let mut entities = Vec::new();
    for &eid in &subtree {
        let name = world.get_name(eid).unwrap_or("").to_string();
        let transform = world.get_transform(eid).copied();

        let mesh = world.get_mesh_renderer(eid).and_then(|mr| {
            let mesh_name = mr.mesh_id
                .and_then(|id| mesh_store.get_name(id))
                .unwrap_or("")
                .to_string();
            if mesh_name.is_empty() && mr.mesh_id.is_none() {
                None
            } else {
                Some(MeshRef { name: mesh_name, visible: mr.visible })
            }
        });

        let material = world.get_material(eid).cloned();
        let light = world.get_light(eid).copied();
        let rigid_body = world.get_rigid_body(eid).copied();
        let collider = world.get_collider(eid).copied();
        let camera = world.get_camera(eid).copied();
        let audio_source = world.get_audio_source(eid).cloned();
        let audio_listener = world.get_audio_listener(eid).copied();
        let ui_element = world.get_ui_element(eid).cloned();
        let canvas = world.get_canvas(eid).copied();
        let animator = world.get_animator(eid).cloned();
        let skeletal_animator = world.get_skeletal_animator(eid).cloned();

        let entity_scripts: Vec<ScriptData> = scripts.iter()
            .filter(|(id, _)| *id == eid)
            .map(|(_, s)| ScriptData { name: s.name().to_string() })
            .collect();

        // Parent index within the subtree (root has None)
        let parent_index = world.get_parent(eid)
            .and_then(|pid| id_to_idx.get(&pid).copied());

        entities.push(EntityData {
            name,
            parent_index,
            transform,
            mesh,
            material,
            light,
            rigid_body,
            collider,
            camera,
            audio_source,
            audio_listener,
            ui_element,
            canvas,
            animator,
            skeletal_animator,
            scripts: entity_scripts,
        });
    }

    let prefab = PrefabData { entities };
    let config = ron::ser::PrettyConfig::default();
    let ron_str = ron::ser::to_string_pretty(&prefab, config)?;
    std::fs::write(path, ron_str)?;
    Ok(())
}

/// Instantiate a prefab into the world. Returns the root entity ID.
pub fn instantiate_prefab(
    world: &mut World,
    mesh_store: &mut MeshStore,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
    script_registry: &[ScriptRegistryEntry],
    device: &wgpu::Device,
    path: &str,
) -> Result<EntityId> {
    let ron_str = std::fs::read_to_string(path)?;
    let prefab: PrefabData = ron::from_str(&ron_str)?;

    if prefab.entities.is_empty() {
        anyhow::bail!("Prefab is empty");
    }

    // Phase 1: Create all entities
    let mut new_ids: Vec<EntityId> = Vec::new();
    for edata in &prefab.entities {
        let eid = world.spawn_entity();
        new_ids.push(eid);
        world.set_name(eid, &edata.name);

        if let Some(t) = &edata.transform {
            world.set_transform(eid, *t);
        }

        if let Some(mref) = &edata.mesh {
            let mesh_id = resolve_mesh(mesh_store, device, &mref.name);
            world.set_mesh_renderer(eid, MeshRenderer {
                mesh_id,
                visible: mref.visible,
            });
        }

        if let Some(mat) = &edata.material {
            world.set_material(eid, mat.clone());
        }
        if let Some(l) = &edata.light {
            world.set_light(eid, *l);
        }
        if let Some(rb) = &edata.rigid_body {
            world.set_rigid_body(eid, *rb);
        }
        if let Some(col) = &edata.collider {
            world.set_collider(eid, *col);
        }
        if let Some(cam) = &edata.camera {
            world.set_camera(eid, *cam);
        }
        if let Some(audio) = &edata.audio_source {
            world.set_audio_source(eid, audio.clone());
        }
        if let Some(al) = &edata.audio_listener {
            world.set_audio_listener(eid, *al);
        }
        if let Some(ui) = &edata.ui_element {
            world.set_ui_element(eid, ui.clone());
        }
        if let Some(cv) = &edata.canvas {
            world.set_canvas(eid, *cv);
        }
        if let Some(anim) = &edata.animator {
            world.set_animator(eid, anim.clone());
        }
        if let Some(sa) = &edata.skeletal_animator {
            world.set_skeletal_animator(eid, sa.clone());
        }

        for sdata in &edata.scripts {
            if let Some(entry) = script_registry.iter().find(|e| e.name == sdata.name) {
                let script = (entry.factory)();
                scripts.push((eid, script));
            } else {
                log::warn!("Prefab script '{}' not in registry, skipping", sdata.name);
            }
        }
    }

    // Phase 2: Restore hierarchy within the prefab
    for (i, edata) in prefab.entities.iter().enumerate() {
        if let Some(parent_idx) = edata.parent_index {
            if parent_idx < new_ids.len() && parent_idx != i {
                world.set_parent(new_ids[i], new_ids[parent_idx]);
            }
        }
    }

    Ok(new_ids[0])
}

/// Resolve a mesh name to MeshStore id, reusing scene.rs logic.
fn resolve_mesh(mesh_store: &mut MeshStore, device: &wgpu::Device, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    // Check if already loaded
    if let Some(id) = mesh_store.find_by_name(name) {
        return Some(id);
    }
    // Try builtin names (Cube, Sphere, etc.) — already in mesh_store at startup
    // For glTF/OBJ meshes, try reloading
    if let Some(rest) = name.strip_prefix("gltf:") {
        if let Some(hash_pos) = rest.find('#') {
            let file_path = &rest[..hash_pos];
            if let Ok(gltf_scene) = super::gltf_loader::load_gltf(file_path) {
                for gm in &gltf_scene.meshes {
                    if mesh_store.find_by_name(&gm.store_name).is_none() {
                        mesh_store.add_named(device, &gm.vertices, &gm.indices, &gm.store_name);
                    }
                }
                return mesh_store.find_by_name(name);
            }
        }
        return None;
    }
    if !name.contains("..") && (name.starts_with("assets/") || name.contains('/')) {
        if let Ok(loaded) = super::obj_loader::load_obj(name) {
            if let Some(lm) = loaded.into_iter().next() {
                let id = mesh_store.add_named(device, &lm.vertices, &lm.indices, name);
                return Some(id);
            }
        }
    }
    log::warn!("Prefab: could not resolve mesh '{}'", name);
    None
}
