use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::core::{World, Transform, MeshRenderer, Material, Light, RigidBody, Collider, CameraComponent, AudioSource, AudioListener, UiElement, Canvas, Animator, SkeletalAnimator, EntityId};
use crate::editor::context::ScriptRegistryEntry;
use crate::renderer::mesh::MeshStore;
use crate::scripting::GameScript;

// ---- Serialization structs ----

#[derive(Serialize, Deserialize)]
pub struct SceneData {
    pub entities: Vec<EntityData>,
}

#[derive(Serialize, Deserialize)]
pub struct EntityData {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<MeshRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<Material>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<Light>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rigid_body: Option<RigidBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collider: Option<Collider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_source: Option<AudioSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_listener: Option<AudioListener>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_element: Option<UiElement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas: Option<Canvas>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animator: Option<Animator>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeletal_animator: Option<SkeletalAnimator>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<ScriptData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshRef {
    pub name: String,
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScriptData {
    pub name: String,
}

// ---- Save ----

pub fn save_scene(
    world: &World,
    mesh_store: &MeshStore,
    scripts: &[(EntityId, Box<dyn GameScript>)],
    path: &str,
) -> Result<()> {
    let mut entities = Vec::new();

    // Build EntityId → index mapping
    let entity_list: Vec<EntityId> = world.iter_entities().collect();
    let id_to_idx: std::collections::HashMap<EntityId, usize> = entity_list.iter()
        .enumerate()
        .map(|(i, &eid)| (eid, i))
        .collect();

    for eid in entity_list {
        let name = world.get_name(eid)
            .unwrap_or("")
            .to_string();

        let transform = world.get_transform(eid).copied();

        let mesh = world.get_mesh_renderer(eid).and_then(|mr| {
            let mesh_name = mr.mesh_id
                .and_then(|id| mesh_store.get_name(id))
                .unwrap_or("")
                .to_string();
            if mesh_name.is_empty() && mr.mesh_id.is_none() {
                None
            } else {
                Some(MeshRef {
                    name: mesh_name,
                    visible: mr.visible,
                })
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

    let scene = SceneData { entities };
    let config = ron::ser::PrettyConfig::default();
    let ron_str = ron::ser::to_string_pretty(&scene, config)?;
    std::fs::write(path, ron_str)?;
    Ok(())
}

// ---- Load ----

pub fn load_scene(
    world: &mut World,
    mesh_store: &mut MeshStore,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
    script_registry: &[ScriptRegistryEntry],
    device: &wgpu::Device,
    path: &str,
) -> Result<()> {
    let ron_str = std::fs::read_to_string(path)?;
    let scene: SceneData = ron::from_str(&ron_str)?;

    // Clear existing world
    let existing: Vec<EntityId> = world.iter_entities().collect();
    for eid in existing {
        world.destroy_entity(eid);
    }
    scripts.clear();

    // Phase 1: Create all entities
    let mut new_ids: Vec<EntityId> = Vec::new();
    for edata in &scene.entities {
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
                log::warn!("Script '{}' not found in registry, skipping", sdata.name);
            }
        }
    }

    // Phase 2: Restore hierarchy
    for (i, edata) in scene.entities.iter().enumerate() {
        if let Some(parent_idx) = edata.parent_index {
            if parent_idx < new_ids.len() && parent_idx != i {
                world.set_parent(new_ids[i], new_ids[parent_idx]);
            }
        }
    }

    Ok(())
}

fn resolve_mesh(mesh_store: &mut MeshStore, device: &wgpu::Device, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }

    // Check if already loaded
    if let Some(id) = mesh_store.find_by_name(name) {
        return Some(id);
    }

    // glTF mesh: "gltf:<path>#<node>/<prim>"
    if name.starts_with("gltf:") {
        if let Some(hash_pos) = name[5..].find('#') {
            let file_path = &name[5..5 + hash_pos];
            match super::gltf_loader::load_gltf(file_path) {
                Ok(gltf_scene) => {
                    for gm in &gltf_scene.meshes {
                        if mesh_store.find_by_name(&gm.store_name).is_none() {
                            mesh_store.add_named(device, &gm.vertices, &gm.indices, &gm.store_name);
                        }
                    }
                    return mesh_store.find_by_name(name);
                }
                Err(e) => {
                    log::warn!("Failed to reload glTF mesh '{}': {}", name, e);
                }
            }
        }
        return None;
    }

    // Try loading OBJ if it's a safe file path
    if name.contains("..") {
        log::error!("Rejected unsafe mesh path: {}", name);
        return None;
    }
    if name.starts_with("assets/") || name.contains('/') {
        match super::obj_loader::load_obj(name) {
            Ok(loaded) => {
                if let Some(lm) = loaded.into_iter().next() {
                    let id = mesh_store.add_named(device, &lm.vertices, &lm.indices, name);
                    log::info!("Loaded mesh from scene: {}", name);
                    return Some(id);
                }
            }
            Err(e) => {
                log::warn!("Failed to load mesh '{}': {}", name, e);
            }
        }
    }

    log::warn!("Could not resolve mesh: {}", name);
    None
}
