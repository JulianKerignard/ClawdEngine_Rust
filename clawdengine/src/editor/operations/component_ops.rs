use crate::core::{self, EntityId, World};
use crate::editor::context::{ComponentKind, EditorContext};
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting::GameScript;

pub(crate) fn process_undo(world: &mut World, ec: &mut EditorContext) {
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

pub(crate) fn process_redo(world: &mut World, ec: &mut EditorContext) {
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

pub(crate) fn process_remove_scripts(
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

pub(crate) fn process_add_script(
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

pub(crate) fn process_texture_assign(
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

pub(crate) fn process_normal_map_assign(
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

pub(crate) fn process_add_component(world: &mut World, ec: &mut EditorContext) {
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
        ComponentKind::Animator => {
            world.set_animator(eid, core::Animator::default());
        }
        ComponentKind::SkeletalAnimator => {
            world.set_skeletal_animator(eid, core::SkeletalAnimator::default());
        }
    }
}
