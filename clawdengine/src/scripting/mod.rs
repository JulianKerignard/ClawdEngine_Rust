#![allow(dead_code, unused_imports)]
pub mod demo_scripts;
pub mod fps_controller;
pub mod math_utils;

pub use demo_scripts::*;
pub use fps_controller::FPSController;
pub use math_utils::{GameRng, lerp, lerp_vec3, distance, distance_squared, look_at_rotation, move_toward};

use crate::core::{EntityId, World, Transform, RigidBody, Material, AudioSource};
use crate::input::Input;
use crate::physics::collision::CollisionEvent;
use crate::physics::raycast::RayHit;
use crate::renderer::mesh::MeshStore;

pub use winit::keyboard::KeyCode;
pub use winit::event::MouseButton;

/// Trait for user-defined game scripts attached to entities.
/// Implement `start` for one-time init, `update` for per-frame logic.
#[allow(unused_variables)]
pub trait GameScript {
    fn name(&self) -> &str { "Script" }
    fn start(&mut self, ctx: &mut ScriptContext) {}
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {}
    fn inspector_ui(&mut self, _ui: &mut egui::Ui) {}
    /// Draw HUD overlay in the Game View (play mode only).
    fn game_ui(&mut self, _ui: &mut egui::Ui) {}
}

/// Tags type stored in World's TypeMap (runtime-only, not in snapshots).
type Tags = Vec<String>;

/// Provides rich access to the World, input, time, and physics queries for scripts.
pub struct ScriptContext<'a> {
    pub entity: EntityId,
    pub world: &'a mut World,
    collision_events: &'a [CollisionEvent],
    input: &'a Input,
    mesh_store: Option<&'a MeshStore>,
    pub time: f32,
    pub dt: f32,
    pending_destroy: Vec<EntityId>,
}

impl<'a> ScriptContext<'a> {
    pub fn new_full(
        entity: EntityId,
        world: &'a mut World,
        events: &'a [CollisionEvent],
        input: &'a Input,
        time: f32,
        dt: f32,
    ) -> Self {
        Self {
            entity,
            world,
            collision_events: events,
            input,
            mesh_store: None,
            time,
            dt,
            pending_destroy: Vec::new(),
        }
    }

    /// Set the optional MeshStore reference for mesh-AABB raycasting.
    pub fn with_mesh_store(mut self, mesh_store: &'a MeshStore) -> Self {
        self.mesh_store = Some(mesh_store);
        self
    }

    /// Take the list of entities to destroy after script execution.
    pub fn take_pending_destroy(&mut self) -> Vec<EntityId> {
        std::mem::take(&mut self.pending_destroy)
    }

    // ================================================================
    // Collision queries
    // ================================================================

    /// Returns collision events involving this entity.
    pub fn collisions(&self) -> impl Iterator<Item = &CollisionEvent> {
        let eid = self.entity;
        self.collision_events.iter().filter(move |e| e.entity == eid)
    }

    // ================================================================
    // Self-entity component access (convenience)
    // ================================================================

    pub fn get_transform(&self) -> Option<&Transform> {
        self.world.get_transform(self.entity)
    }

    pub fn get_transform_mut(&mut self) -> Option<&mut Transform> {
        self.world.get_transform_mut(self.entity)
    }

    pub fn get_rigid_body(&self) -> Option<&RigidBody> {
        self.world.get_rigid_body(self.entity)
    }

    pub fn get_rigid_body_mut(&mut self) -> Option<&mut RigidBody> {
        self.world.get_rigid_body_mut(self.entity)
    }

    pub fn get_material(&self) -> Option<&Material> {
        self.world.get_material(self.entity)
    }

    pub fn get_material_mut(&mut self) -> Option<&mut Material> {
        self.world.get_material_mut(self.entity)
    }

    pub fn get_audio_source(&self) -> Option<&AudioSource> {
        self.world.get_audio_source(self.entity)
    }

    pub fn get_audio_source_mut(&mut self) -> Option<&mut AudioSource> {
        self.world.get_audio_source_mut(self.entity)
    }

    pub fn play_audio(&mut self) {
        if let Some(a) = self.world.get_audio_source_mut(self.entity) {
            a.is_playing = true;
        }
    }

    pub fn stop_audio(&mut self) {
        if let Some(a) = self.world.get_audio_source_mut(self.entity) {
            a.is_playing = false;
        }
    }

    // ================================================================
    // Input
    // ================================================================

    pub fn is_key_held(&self, key: KeyCode) -> bool {
        self.input.is_key_held(key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.input.is_key_pressed(key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.input.is_key_released(key)
    }

    pub fn is_mouse_held(&self, button: MouseButton) -> bool {
        self.input.is_mouse_held(button)
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.input.is_mouse_pressed(button)
    }

    pub fn mouse_position(&self) -> [f32; 2] {
        self.input.mouse_position()
    }

    pub fn mouse_delta(&self) -> [f32; 2] {
        self.input.mouse_delta()
    }

    pub fn scroll_delta(&self) -> [f32; 2] {
        self.input.scroll_delta()
    }

    // ================================================================
    // Cross-entity access
    // ================================================================

    /// Find the first entity with the given name.
    pub fn find_by_name(&self, name: &str) -> Option<EntityId> {
        self.world.iter_entities().find(|&eid| {
            self.world.get_name(eid).is_some_and(|n| n == name)
        })
    }

    /// Find all entities whose name contains the given substring.
    pub fn find_all_by_name(&self, pattern: &str) -> Vec<EntityId> {
        self.world.iter_entities().filter(|&eid| {
            self.world.get_name(eid).is_some_and(|n| n.contains(pattern))
        }).collect()
    }

    /// Get another entity's transform (read-only).
    pub fn get_entity_transform(&self, target: EntityId) -> Option<&Transform> {
        self.world.get_transform(target)
    }

    /// Get another entity's transform (mutable).
    pub fn get_entity_transform_mut(&mut self, target: EntityId) -> Option<&mut Transform> {
        self.world.get_transform_mut(target)
    }

    /// Get another entity's material (mutable).
    pub fn get_entity_material_mut(&mut self, target: EntityId) -> Option<&mut Material> {
        self.world.get_material_mut(target)
    }

    /// Get another entity's rigid body (mutable).
    pub fn get_entity_rigid_body_mut(&mut self, target: EntityId) -> Option<&mut RigidBody> {
        self.world.get_rigid_body_mut(target)
    }

    /// Get another entity's name.
    pub fn get_entity_name(&self, target: EntityId) -> Option<&str> {
        self.world.get_name(target)
    }

    /// Check if an entity is still alive.
    pub fn is_alive(&self, target: EntityId) -> bool {
        self.world.is_alive(target)
    }

    /// Spawn a new entity (returns its EntityId).
    pub fn spawn_entity(&mut self) -> EntityId {
        self.world.spawn_entity()
    }

    /// Mark an entity for deferred destruction (applied after all scripts run).
    pub fn destroy_entity(&mut self, target: EntityId) {
        self.pending_destroy.push(target);
    }

    // ================================================================
    // Tags (runtime-only, via World TypeMap)
    // ================================================================

    /// Add a tag to an entity.
    pub fn add_tag(&mut self, target: EntityId, tag: &str) {
        let mut tags = self.world.get_custom::<Tags>(target)
            .cloned()
            .unwrap_or_default();
        if !tags.iter().any(|t| t == tag) {
            tags.push(tag.to_string());
        }
        self.world.add_custom(target, tags);
    }

    /// Check if an entity has a specific tag.
    pub fn has_tag(&self, target: EntityId, tag: &str) -> bool {
        self.world.get_custom::<Tags>(target)
            .is_some_and(|tags| tags.iter().any(|t| t == tag))
    }

    /// Find all entities with a given tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<EntityId> {
        self.world.iter_entities().filter(|&eid| self.has_tag(eid, tag)).collect()
    }

    /// Remove a tag from an entity.
    pub fn remove_tag(&mut self, target: EntityId, tag: &str) {
        if let Some(mut tags) = self.world.get_custom::<Tags>(target).cloned() {
            tags.retain(|t| t != tag);
            self.world.add_custom(target, tags);
        }
    }

    // ================================================================
    // Raycasting
    // ================================================================

    /// Cast a ray from a point in a direction. Returns the closest collider hit.
    pub fn raycast(&self, origin: glam::Vec3, direction: glam::Vec3, max_distance: f32) -> Option<RayHit> {
        crate::physics::raycast::raycast(self.world, origin, direction, max_distance)
    }

    /// Cast a ray and return all hits sorted by distance.
    pub fn raycast_all(&self, origin: glam::Vec3, direction: glam::Vec3, max_distance: f32) -> Vec<RayHit> {
        crate::physics::raycast::raycast_all(self.world, origin, direction, max_distance)
    }

    /// Cast a ray forward from this entity's position and orientation.
    pub fn raycast_forward(&self, max_distance: f32) -> Option<RayHit> {
        let t = self.world.get_transform(self.entity)?;
        let forward = t.rotation * glam::Vec3::NEG_Z;
        crate::physics::raycast::raycast(self.world, t.position, forward, max_distance)
    }

    /// Cast a ray against mesh AABBs of all visible entities (no Collider needed).
    /// Returns the closest hit, or `None` if MeshStore is unavailable.
    pub fn raycast_mesh(&self, origin: glam::Vec3, direction: glam::Vec3, max_distance: f32) -> Option<RayHit> {
        let ms = self.mesh_store?;
        crate::physics::raycast::raycast_mesh_aabb(self.world, ms, origin, direction, max_distance)
    }

    /// Cast a ray against all visible mesh AABBs. Returns all hits sorted by distance.
    /// Returns empty Vec if MeshStore is unavailable.
    pub fn raycast_mesh_all(&self, origin: glam::Vec3, direction: glam::Vec3, max_distance: f32) -> Vec<RayHit> {
        match self.mesh_store {
            Some(ms) => crate::physics::raycast::raycast_mesh_aabb_all(self.world, ms, origin, direction, max_distance),
            None => Vec::new(),
        }
    }

    /// Cast a ray forward from this entity against mesh AABBs.
    pub fn raycast_mesh_forward(&self, max_distance: f32) -> Option<RayHit> {
        let t = self.world.get_transform(self.entity)?;
        let forward = t.rotation * glam::Vec3::NEG_Z;
        let ms = self.mesh_store?;
        crate::physics::raycast::raycast_mesh_aabb(self.world, ms, t.position, forward, max_distance)
    }
}
