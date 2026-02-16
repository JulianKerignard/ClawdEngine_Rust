use std::any::{Any, TypeId};
use std::collections::HashMap;

use super::entity::EntityId;
use super::components::*;
use super::skeleton::SkeletalAnimator;

#[derive(Clone)]
pub struct WorldSnapshot {
    alive: Vec<bool>,
    names: Vec<String>,
    transforms: Vec<Option<Transform>>,
    mesh_renderers: Vec<Option<MeshRenderer>>,
    materials: Vec<Option<Material>>,
    lights: Vec<Option<Light>>,
    rigid_bodies: Vec<Option<RigidBody>>,
    colliders: Vec<Option<Collider>>,
    cameras: Vec<Option<CameraComponent>>,
    audio_sources: Vec<Option<AudioSource>>,
    audio_listeners: Vec<Option<AudioListener>>,
    ui_elements: Vec<Option<UiElement>>,
    canvases: Vec<Option<Canvas>>,
    animators: Vec<Option<Animator>>,
    skeletal_animators: Vec<Option<SkeletalAnimator>>,
    parents: Vec<Option<EntityId>>,
    children: Vec<Option<Vec<EntityId>>>,
}

fn combine_transforms(parent: &Transform, child: &Transform) -> Transform {
    Transform {
        position: parent.position + parent.rotation * (parent.scale * child.position),
        rotation: parent.rotation * child.rotation,
        scale: parent.scale * child.scale,
    }
}

pub struct World {
    // Entity management
    generations: Vec<u32>,
    alive: Vec<bool>,
    free_list: Vec<u32>,
    names: Vec<String>,

    // SoA component storage
    transforms: Vec<Option<Transform>>,
    mesh_renderers: Vec<Option<MeshRenderer>>,
    materials: Vec<Option<Material>>,
    lights: Vec<Option<Light>>,
    rigid_bodies: Vec<Option<RigidBody>>,
    colliders: Vec<Option<Collider>>,
    cameras: Vec<Option<CameraComponent>>,
    audio_sources: Vec<Option<AudioSource>>,
    audio_listeners: Vec<Option<AudioListener>>,
    ui_elements: Vec<Option<UiElement>>,
    canvases: Vec<Option<Canvas>>,
    animators: Vec<Option<Animator>>,
    skeletal_animators: Vec<Option<SkeletalAnimator>>,

    // Hierarchy
    parents: Vec<Option<EntityId>>,
    children: Vec<Option<Vec<EntityId>>>,

    // TypeMap for custom components
    custom: HashMap<TypeId, Vec<Option<Box<dyn Any>>>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            alive: Vec::new(),
            free_list: Vec::new(),
            names: Vec::new(),
            transforms: Vec::new(),
            mesh_renderers: Vec::new(),
            materials: Vec::new(),
            lights: Vec::new(),
            rigid_bodies: Vec::new(),
            colliders: Vec::new(),
            cameras: Vec::new(),
            audio_sources: Vec::new(),
            audio_listeners: Vec::new(),
            ui_elements: Vec::new(),
            canvases: Vec::new(),
            animators: Vec::new(),
            skeletal_animators: Vec::new(),
            parents: Vec::new(),
            children: Vec::new(),
            custom: HashMap::new(),
        }
    }

    // ---- Entity lifecycle ----

    pub fn spawn_entity(&mut self) -> EntityId {
        if let Some(index) = self.free_list.pop() {
            let idx = index as usize;
            self.generations[idx] += 1;
            self.alive[idx] = true;
            self.names[idx] = String::new();
            self.transforms[idx] = None;
            self.mesh_renderers[idx] = None;
            self.materials[idx] = None;
            self.lights[idx] = None;
            self.rigid_bodies[idx] = None;
            self.colliders[idx] = None;
            self.cameras[idx] = None;
            self.audio_sources[idx] = None;
            self.audio_listeners[idx] = None;
            self.ui_elements[idx] = None;
            self.canvases[idx] = None;
            self.animators[idx] = None;
            self.skeletal_animators[idx] = None;
            self.parents[idx] = None;
            self.children[idx] = None;
            for vec in self.custom.values_mut() {
                vec[idx] = None;
            }
            EntityId::new(index, self.generations[idx])
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            self.alive.push(true);
            self.names.push(String::new());
            self.transforms.push(None);
            self.mesh_renderers.push(None);
            self.materials.push(None);
            self.lights.push(None);
            self.rigid_bodies.push(None);
            self.colliders.push(None);
            self.cameras.push(None);
            self.audio_sources.push(None);
            self.audio_listeners.push(None);
            self.ui_elements.push(None);
            self.canvases.push(None);
            self.animators.push(None);
            self.skeletal_animators.push(None);
            self.parents.push(None);
            self.children.push(None);
            for vec in self.custom.values_mut() {
                vec.push(None);
            }
            EntityId::new(index, 0)
        }
    }

    pub fn destroy_entity(&mut self, id: EntityId) {
        if !self.is_alive(id) {
            return;
        }
        let idx = id.index as usize;
        self.alive[idx] = false;
        self.names[idx].clear();
        self.transforms[idx] = None;
        self.mesh_renderers[idx] = None;
        self.materials[idx] = None;
        self.lights[idx] = None;
        self.rigid_bodies[idx] = None;
        self.colliders[idx] = None;
        self.cameras[idx] = None;
        self.audio_sources[idx] = None;
        self.audio_listeners[idx] = None;
        self.ui_elements[idx] = None;
        self.canvases[idx] = None;
        self.animators[idx] = None;
        self.skeletal_animators[idx] = None;
        // Detach from parent
        if let Some(parent_id) = self.parents[idx].take() {
            if self.is_alive(parent_id) {
                let pidx = parent_id.index as usize;
                if let Some(ref mut ch) = self.children[pidx] {
                    ch.retain(|&c| c != id);
                }
            }
        }
        // Detach children (they become root, cascade handled by editor)
        if let Some(child_ids) = self.children[idx].take() {
            for child_id in child_ids {
                if self.is_alive(child_id) {
                    self.parents[child_id.index as usize] = None;
                }
            }
        }
        for vec in self.custom.values_mut() {
            vec[idx] = None;
        }
        self.free_list.push(id.index);
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        let idx = id.index as usize;
        idx < self.alive.len() && self.generations[idx] == id.generation && self.alive[idx]
    }

    pub fn entity_count(&self) -> usize {
        self.alive.iter().filter(|&&a| a).count()
    }

    pub fn set_name(&mut self, id: EntityId, name: impl Into<String>) {
        if self.is_alive(id) {
            self.names[id.index as usize] = name.into();
        }
    }

    pub fn get_name(&self, id: EntityId) -> Option<&str> {
        if self.is_alive(id) {
            let name = &self.names[id.index as usize];
            if name.is_empty() { None } else { Some(name) }
        } else {
            None
        }
    }

    pub fn get_name_mut(&mut self, id: EntityId) -> Option<&mut String> {
        if self.is_alive(id) {
            Some(&mut self.names[id.index as usize])
        } else {
            None
        }
    }

    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| EntityId::new(i as u32, self.generations[i]))
    }

    // ---- Hierarchy ----

    pub fn set_parent(&mut self, child: EntityId, parent: EntityId) {
        if !self.is_alive(child) || !self.is_alive(parent) { return; }
        if child == parent { return; }
        if self.is_ancestor(child, parent) { return; }

        // Remove from old parent
        if let Some(old_parent) = self.parents[child.index as usize] {
            if self.is_alive(old_parent) {
                if let Some(ref mut ch) = self.children[old_parent.index as usize] {
                    ch.retain(|&c| c != child);
                }
            }
        }

        self.parents[child.index as usize] = Some(parent);
        let pidx = parent.index as usize;
        match &mut self.children[pidx] {
            Some(ch) => ch.push(child),
            None => self.children[pidx] = Some(vec![child]),
        }
    }

    pub fn remove_parent(&mut self, child: EntityId) {
        if !self.is_alive(child) { return; }
        if let Some(parent_id) = self.parents[child.index as usize].take() {
            if self.is_alive(parent_id) {
                if let Some(ref mut ch) = self.children[parent_id.index as usize] {
                    ch.retain(|&c| c != child);
                }
            }
        }
    }

    pub fn get_parent(&self, id: EntityId) -> Option<EntityId> {
        if !self.is_alive(id) { return None; }
        self.parents[id.index as usize]
    }

    pub fn get_children(&self, id: EntityId) -> &[EntityId] {
        if !self.is_alive(id) { return &[]; }
        self.children[id.index as usize].as_deref().unwrap_or(&[])
    }

    pub fn is_ancestor(&self, ancestor: EntityId, id: EntityId) -> bool {
        let mut current = self.get_parent(id);
        while let Some(pid) = current {
            if pid == ancestor { return true; }
            current = self.get_parent(pid);
        }
        false
    }

    pub fn get_world_transform(&self, id: EntityId) -> Option<Transform> {
        let local = self.get_transform(id)?;
        match self.get_parent(id) {
            None => Some(*local),
            Some(parent_id) => {
                let parent_world = self.get_world_transform(parent_id)?;
                Some(combine_transforms(&parent_world, local))
            }
        }
    }

    // ---- Transform ----

    pub fn set_transform(&mut self, id: EntityId, t: Transform) {
        if self.is_alive(id) { self.transforms[id.index as usize] = Some(t); }
    }

    pub fn get_transform(&self, id: EntityId) -> Option<&Transform> {
        if !self.is_alive(id) { return None; }
        self.transforms[id.index as usize].as_ref()
    }

    pub fn get_transform_mut(&mut self, id: EntityId) -> Option<&mut Transform> {
        if !self.is_alive(id) { return None; }
        self.transforms[id.index as usize].as_mut()
    }

    pub fn remove_transform(&mut self, id: EntityId) {
        if self.is_alive(id) { self.transforms[id.index as usize] = None; }
    }

    pub fn transforms_iter(&self) -> impl Iterator<Item = (EntityId, &Transform)> + '_ {
        self.transforms.iter().enumerate().filter_map(|(i, t)| {
            if self.alive[i] {
                t.as_ref().map(|t| (EntityId::new(i as u32, self.generations[i]), t))
            } else {
                None
            }
        })
    }

    // ---- MeshRenderer ----

    pub fn set_mesh_renderer(&mut self, id: EntityId, m: MeshRenderer) {
        if self.is_alive(id) { self.mesh_renderers[id.index as usize] = Some(m); }
    }

    pub fn get_mesh_renderer(&self, id: EntityId) -> Option<&MeshRenderer> {
        if !self.is_alive(id) { return None; }
        self.mesh_renderers[id.index as usize].as_ref()
    }

    pub fn get_mesh_renderer_mut(&mut self, id: EntityId) -> Option<&mut MeshRenderer> {
        if !self.is_alive(id) { return None; }
        self.mesh_renderers[id.index as usize].as_mut()
    }

    pub fn remove_mesh_renderer(&mut self, id: EntityId) {
        if self.is_alive(id) { self.mesh_renderers[id.index as usize] = None; }
    }

    // ---- Material ----

    pub fn set_material(&mut self, id: EntityId, m: Material) {
        if self.is_alive(id) { self.materials[id.index as usize] = Some(m); }
    }

    pub fn get_material(&self, id: EntityId) -> Option<&Material> {
        if !self.is_alive(id) { return None; }
        self.materials[id.index as usize].as_ref()
    }

    pub fn get_material_mut(&mut self, id: EntityId) -> Option<&mut Material> {
        if !self.is_alive(id) { return None; }
        self.materials[id.index as usize].as_mut()
    }

    pub fn remove_material(&mut self, id: EntityId) {
        if self.is_alive(id) { self.materials[id.index as usize] = None; }
    }

    // ---- Light ----

    pub fn set_light(&mut self, id: EntityId, l: Light) {
        if self.is_alive(id) { self.lights[id.index as usize] = Some(l); }
    }

    pub fn get_light(&self, id: EntityId) -> Option<&Light> {
        if !self.is_alive(id) { return None; }
        self.lights[id.index as usize].as_ref()
    }

    pub fn get_light_mut(&mut self, id: EntityId) -> Option<&mut Light> {
        if !self.is_alive(id) { return None; }
        self.lights[id.index as usize].as_mut()
    }

    pub fn remove_light(&mut self, id: EntityId) {
        if self.is_alive(id) { self.lights[id.index as usize] = None; }
    }

    pub fn lights_iter(&self) -> impl Iterator<Item = (EntityId, &Light)> + '_ {
        self.lights.iter().enumerate().filter_map(|(i, l)| {
            if self.alive[i] {
                l.as_ref().map(|l| (EntityId::new(i as u32, self.generations[i]), l))
            } else {
                None
            }
        })
    }

    // ---- RigidBody ----

    pub fn set_rigid_body(&mut self, id: EntityId, r: RigidBody) {
        if self.is_alive(id) { self.rigid_bodies[id.index as usize] = Some(r); }
    }

    pub fn get_rigid_body(&self, id: EntityId) -> Option<&RigidBody> {
        if !self.is_alive(id) { return None; }
        self.rigid_bodies[id.index as usize].as_ref()
    }

    pub fn get_rigid_body_mut(&mut self, id: EntityId) -> Option<&mut RigidBody> {
        if !self.is_alive(id) { return None; }
        self.rigid_bodies[id.index as usize].as_mut()
    }

    pub fn remove_rigid_body(&mut self, id: EntityId) {
        if self.is_alive(id) { self.rigid_bodies[id.index as usize] = None; }
    }

    #[allow(dead_code)]
    pub fn rigid_bodies_iter(&self) -> impl Iterator<Item = (EntityId, &RigidBody)> + '_ {
        self.rigid_bodies.iter().enumerate().filter_map(|(i, rb)| {
            if self.alive[i] {
                rb.as_ref().map(|rb| (EntityId::new(i as u32, self.generations[i]), rb))
            } else {
                None
            }
        })
    }

    // ---- Collider ----

    pub fn set_collider(&mut self, id: EntityId, c: Collider) {
        if self.is_alive(id) { self.colliders[id.index as usize] = Some(c); }
    }

    pub fn get_collider(&self, id: EntityId) -> Option<&Collider> {
        if !self.is_alive(id) { return None; }
        self.colliders[id.index as usize].as_ref()
    }

    pub fn get_collider_mut(&mut self, id: EntityId) -> Option<&mut Collider> {
        if !self.is_alive(id) { return None; }
        self.colliders[id.index as usize].as_mut()
    }

    pub fn remove_collider(&mut self, id: EntityId) {
        if self.is_alive(id) { self.colliders[id.index as usize] = None; }
    }

    // ---- Camera ----

    pub fn set_camera(&mut self, id: EntityId, c: CameraComponent) {
        if self.is_alive(id) { self.cameras[id.index as usize] = Some(c); }
    }

    pub fn get_camera(&self, id: EntityId) -> Option<&CameraComponent> {
        if !self.is_alive(id) { return None; }
        self.cameras[id.index as usize].as_ref()
    }

    pub fn get_camera_mut(&mut self, id: EntityId) -> Option<&mut CameraComponent> {
        if !self.is_alive(id) { return None; }
        self.cameras[id.index as usize].as_mut()
    }

    pub fn remove_camera(&mut self, id: EntityId) {
        if self.is_alive(id) { self.cameras[id.index as usize] = None; }
    }

    // ---- AudioSource ----

    pub fn set_audio_source(&mut self, id: EntityId, a: AudioSource) {
        if self.is_alive(id) { self.audio_sources[id.index as usize] = Some(a); }
    }

    pub fn get_audio_source(&self, id: EntityId) -> Option<&AudioSource> {
        if !self.is_alive(id) { return None; }
        self.audio_sources[id.index as usize].as_ref()
    }

    pub fn get_audio_source_mut(&mut self, id: EntityId) -> Option<&mut AudioSource> {
        if !self.is_alive(id) { return None; }
        self.audio_sources[id.index as usize].as_mut()
    }

    pub fn remove_audio_source(&mut self, id: EntityId) {
        if self.is_alive(id) { self.audio_sources[id.index as usize] = None; }
    }

    // ---- AudioListener ----

    pub fn set_audio_listener(&mut self, id: EntityId, a: AudioListener) {
        if self.is_alive(id) { self.audio_listeners[id.index as usize] = Some(a); }
    }

    pub fn get_audio_listener(&self, id: EntityId) -> Option<&AudioListener> {
        if !self.is_alive(id) { return None; }
        self.audio_listeners[id.index as usize].as_ref()
    }

    pub fn get_audio_listener_mut(&mut self, id: EntityId) -> Option<&mut AudioListener> {
        if !self.is_alive(id) { return None; }
        self.audio_listeners[id.index as usize].as_mut()
    }

    pub fn remove_audio_listener(&mut self, id: EntityId) {
        if self.is_alive(id) { self.audio_listeners[id.index as usize] = None; }
    }

    // ---- UiElement ----

    pub fn set_ui_element(&mut self, id: EntityId, el: UiElement) {
        if self.is_alive(id) { self.ui_elements[id.index as usize] = Some(el); }
    }

    pub fn get_ui_element(&self, id: EntityId) -> Option<&UiElement> {
        if !self.is_alive(id) { return None; }
        self.ui_elements[id.index as usize].as_ref()
    }

    pub fn get_ui_element_mut(&mut self, id: EntityId) -> Option<&mut UiElement> {
        if !self.is_alive(id) { return None; }
        self.ui_elements[id.index as usize].as_mut()
    }

    pub fn remove_ui_element(&mut self, id: EntityId) {
        if self.is_alive(id) { self.ui_elements[id.index as usize] = None; }
    }

    // ---- Canvas ----

    pub fn set_canvas(&mut self, id: EntityId, c: Canvas) {
        if self.is_alive(id) { self.canvases[id.index as usize] = Some(c); }
    }

    pub fn get_canvas(&self, id: EntityId) -> Option<&Canvas> {
        if !self.is_alive(id) { return None; }
        self.canvases[id.index as usize].as_ref()
    }

    pub fn get_canvas_mut(&mut self, id: EntityId) -> Option<&mut Canvas> {
        if !self.is_alive(id) { return None; }
        self.canvases[id.index as usize].as_mut()
    }

    pub fn remove_canvas(&mut self, id: EntityId) {
        if self.is_alive(id) { self.canvases[id.index as usize] = None; }
    }

    // ---- Animator ----

    pub fn set_animator(&mut self, id: EntityId, a: Animator) {
        if self.is_alive(id) { self.animators[id.index as usize] = Some(a); }
    }

    pub fn get_animator(&self, id: EntityId) -> Option<&Animator> {
        if !self.is_alive(id) { return None; }
        self.animators[id.index as usize].as_ref()
    }

    pub fn get_animator_mut(&mut self, id: EntityId) -> Option<&mut Animator> {
        if !self.is_alive(id) { return None; }
        self.animators[id.index as usize].as_mut()
    }

    pub fn remove_animator(&mut self, id: EntityId) {
        if self.is_alive(id) { self.animators[id.index as usize] = None; }
    }

    // ---- SkeletalAnimator ----

    pub fn set_skeletal_animator(&mut self, id: EntityId, a: SkeletalAnimator) {
        if self.is_alive(id) { self.skeletal_animators[id.index as usize] = Some(a); }
    }

    pub fn get_skeletal_animator(&self, id: EntityId) -> Option<&SkeletalAnimator> {
        if !self.is_alive(id) { return None; }
        self.skeletal_animators[id.index as usize].as_ref()
    }

    pub fn get_skeletal_animator_mut(&mut self, id: EntityId) -> Option<&mut SkeletalAnimator> {
        if !self.is_alive(id) { return None; }
        self.skeletal_animators[id.index as usize].as_mut()
    }

    pub fn remove_skeletal_animator(&mut self, id: EntityId) {
        if self.is_alive(id) { self.skeletal_animators[id.index as usize] = None; }
    }

    // ---- TypeMap: custom components ----

    pub fn add_custom<T: Any>(&mut self, id: EntityId, comp: T) {
        if !self.is_alive(id) { return; }
        let type_id = TypeId::of::<T>();
        let vec = self.custom.entry(type_id).or_insert_with(|| {
            let mut v: Vec<Option<Box<dyn Any>>> = Vec::new();
            v.resize_with(self.alive.len(), || None);
            v
        });
        vec[id.index as usize] = Some(Box::new(comp));
    }

    pub fn get_custom<T: Any>(&self, id: EntityId) -> Option<&T> {
        if !self.is_alive(id) { return None; }
        self.custom.get(&TypeId::of::<T>())
            .and_then(|vec| vec[id.index as usize].as_ref())
            .and_then(|b| b.downcast_ref::<T>())
    }

    pub fn get_custom_mut<T: Any>(&mut self, id: EntityId) -> Option<&mut T> {
        if !self.is_alive(id) { return None; }
        self.custom.get_mut(&TypeId::of::<T>())
            .and_then(|vec| vec[id.index as usize].as_mut())
            .and_then(|b| b.downcast_mut::<T>())
    }

    pub fn remove_custom<T: Any>(&mut self, id: EntityId) {
        if !self.is_alive(id) { return; }
        if let Some(vec) = self.custom.get_mut(&TypeId::of::<T>()) {
            vec[id.index as usize] = None;
        }
    }

    // ---- Snapshot/Restore (Story 5.2) ----

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            alive: self.alive.clone(),
            names: self.names.clone(),
            transforms: self.transforms.clone(),
            mesh_renderers: self.mesh_renderers.clone(),
            materials: self.materials.clone(),
            lights: self.lights.clone(),
            rigid_bodies: self.rigid_bodies.clone(),
            colliders: self.colliders.clone(),
            cameras: self.cameras.clone(),
            audio_sources: self.audio_sources.clone(),
            audio_listeners: self.audio_listeners.clone(),
            ui_elements: self.ui_elements.clone(),
            canvases: self.canvases.clone(),
            animators: self.animators.clone(),
            skeletal_animators: self.skeletal_animators.clone(),
            parents: self.parents.clone(),
            children: self.children.clone(),
        }
    }

    pub fn restore(&mut self, snap: WorldSnapshot) {
        self.alive = snap.alive;
        self.names = snap.names;
        self.transforms = snap.transforms;
        self.mesh_renderers = snap.mesh_renderers;
        self.materials = snap.materials;
        self.lights = snap.lights;
        self.rigid_bodies = snap.rigid_bodies;
        self.colliders = snap.colliders;
        self.cameras = snap.cameras;
        self.audio_sources = snap.audio_sources;
        self.audio_listeners = snap.audio_listeners;
        self.ui_elements = snap.ui_elements;
        self.canvases = snap.canvases;
        self.animators = snap.animators;
        self.skeletal_animators = snap.skeletal_animators;
        self.parents = snap.parents;
        self.children = snap.children;
        // Rebuild free list from alive flags
        self.free_list.clear();
        for (i, &alive) in self.alive.iter().enumerate() {
            if !alive {
                self.free_list.push(i as u32);
            }
        }
    }
}
