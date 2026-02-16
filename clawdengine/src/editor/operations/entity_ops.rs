use crate::core::{self, EntityId, World};
use crate::editor::context::{EditorContext, SpawnRequest};
use crate::scripting::GameScript;

pub(crate) fn process_spawn(world: &mut World, ec: &mut EditorContext) {
    let Some(spawn) = ec.pending_spawn.take() else {
        return;
    };
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    let cube_mesh = ec.builtin_meshes.cube;
    let sphere_mesh = ec.builtin_meshes.sphere;
    let plane_mesh = ec.builtin_meshes.plane;
    let cylinder_mesh = ec.builtin_meshes.cylinder;
    let capsule_mesh = ec.builtin_meshes.capsule;
    let cone_mesh = ec.builtin_meshes.cone;

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
        SpawnRequest::Plane => {
            let id = world.spawn_entity();
            world.set_name(id, "Plane");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(id, core::MeshRenderer { mesh_id: Some(plane_mesh), visible: true });
            id
        }
        SpawnRequest::Cylinder => {
            let id = world.spawn_entity();
            world.set_name(id, "Cylinder");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(id, core::MeshRenderer { mesh_id: Some(cylinder_mesh), visible: true });
            id
        }
        SpawnRequest::Capsule => {
            let id = world.spawn_entity();
            world.set_name(id, "Capsule");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(id, core::MeshRenderer { mesh_id: Some(capsule_mesh), visible: true });
            id
        }
        SpawnRequest::Cone => {
            let id = world.spawn_entity();
            world.set_name(id, "Cone");
            world.set_transform(id, core::Transform::default());
            world.set_material(id, core::Material::default());
            world.set_mesh_renderer(id, core::MeshRenderer { mesh_id: Some(cone_mesh), visible: true });
            id
        }
        SpawnRequest::Sun => {
            let id = world.spawn_entity();
            world.set_name(id, "Sun");
            let rotation = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                -50_f32.to_radians(),
                30_f32.to_radians(),
                0.0,
            );
            world.set_transform(id, core::Transform {
                position: glam::Vec3::new(0.0, 10.0, 0.0),
                rotation,
                ..Default::default()
            });
            world.set_light(id, core::Light {
                kind: core::LightKind::Directional,
                color: glam::Vec3::new(1.0, 0.95, 0.85),
                intensity: 1.5,
                ..Default::default()
            });
            id
        }
        SpawnRequest::PointLight => {
            let id = world.spawn_entity();
            world.set_name(id, "Point Light");
            world.set_transform(id, core::Transform {
                position: glam::Vec3::new(0.0, 3.0, 0.0),
                ..Default::default()
            });
            world.set_light(id, core::Light {
                kind: core::LightKind::Point,
                color: glam::Vec3::ONE,
                intensity: 1.5,
                range: 10.0,
                ..Default::default()
            });
            id
        }
        SpawnRequest::SpotLight => {
            let id = world.spawn_entity();
            world.set_name(id, "Spot Light");
            let rotation = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                -90_f32.to_radians(),
                0.0,
                0.0,
            );
            world.set_transform(id, core::Transform {
                position: glam::Vec3::new(0.0, 5.0, 0.0),
                rotation,
                ..Default::default()
            });
            world.set_light(id, core::Light {
                kind: core::LightKind::Spot,
                color: glam::Vec3::new(1.0, 1.0, 0.9),
                intensity: 2.0,
                range: 15.0,
                inner_angle: 20_f32.to_radians(),
                outer_angle: 35_f32.to_radians(),
            });
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

pub(crate) fn process_delete(
    world: &mut World,
    ec: &mut EditorContext,
    scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
) {
    if ec.pending_delete.is_empty() {
        return;
    }
    let to_delete = std::mem::take(&mut ec.pending_delete);
    ec.undo_stack
        .push(world.snapshot(), ec.selected_entities.clone());

    let mut all = Vec::new();
    for id in &to_delete {
        collect_descendants(world, *id, &mut all);
    }
    all.reverse();
    // Remove scripts attached to deleted entities
    scripts.retain(|(id, _)| !all.contains(id));
    for id in all {
        world.destroy_entity(id);
    }
    ec.deselect_all();
}

pub(crate) fn process_reparent(world: &mut World, ec: &mut EditorContext) {
    let Some((child, new_parent)) = ec.pending_reparent.take() else { return; };
    ec.undo_stack.push(world.snapshot(), ec.selected_entities.clone());
    match new_parent {
        Some(parent) => world.set_parent(child, parent),
        None => world.remove_parent(child),
    }
}

pub(crate) fn process_duplicate(
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
        if let Some(anim) = world.get_animator(*src_id) {
            world.set_animator(new_id, anim.clone());
        }
        if let Some(sa) = world.get_skeletal_animator(*src_id) {
            world.set_skeletal_animator(new_id, sa.clone());
        }

        id_map.insert(*src_id, new_id);

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
