pub mod collision;
pub mod raycast;

use glam::Vec3;

use crate::core::{ColliderShape, EntityId, World};
use collision::{
    CollisionEvent, CollisionState, Contact,
    aabb_contact, sphere_sphere_contact, sphere_aabb_contact,
    resolve_impulse, correct_positions,
};

const GRAVITY: f32 = 9.81;
const GROUND_Y: f32 = 0.0;

struct ColBody {
    eid: EntityId,
    world_min: Vec3,
    world_max: Vec3,
    has_rb: bool,
    is_trigger: bool,
    restitution: f32,
    friction: f32,
    shape: ColliderShape,
    world_center: Vec3,
    world_radius: f32,
}

pub struct PhysicsSystem;

impl PhysicsSystem {
    pub fn step(
        world: &mut World,
        collision_state: &mut CollisionState,
        dt: f32,
    ) -> Vec<CollisionEvent> {
        let entities: Vec<EntityId> = world.iter_entities().collect();

        // 1. Gravity + integrate (entities with RigidBody)
        for &eid in &entities {
            let Some(rb) = world.get_rigid_body(eid) else { continue; };
            let mut velocity = rb.velocity;
            if rb.gravity_enabled {
                velocity.y -= GRAVITY * dt;
            }
            if let Some(t) = world.get_transform_mut(eid) {
                t.position += velocity * dt;
            }
            if let Some(rb) = world.get_rigid_body_mut(eid) {
                rb.velocity = velocity;
            }
        }

        // 2. Collect world AABBs for entities with Collider
        let mut bodies: Vec<ColBody> = Vec::new();
        for &eid in &entities {
            let collider = match world.get_collider(eid) {
                Some(c) => *c,
                None => continue,
            };
            let transform = match world.get_transform(eid) {
                Some(t) => t,
                None => continue,
            };
            let wt = world.get_world_transform(eid).unwrap_or(*transform);
            let (world_min, world_max) = collider.world_aabb(wt.position, wt.rotation, wt.scale);
            let world_center = (world_min + world_max) * 0.5;
            let max_scale = wt.scale.x.max(wt.scale.y).max(wt.scale.z);
            let world_radius = collider.radius * max_scale;

            bodies.push(ColBody {
                eid,
                world_min,
                world_max,
                has_rb: world.get_rigid_body(eid).is_some(),
                is_trigger: collider.is_trigger,
                restitution: collider.restitution,
                friction: collider.friction,
                shape: collider.shape,
                world_center,
                world_radius,
            });
        }

        // 3. Broad phase N² — detect contacts
        let mut contacts: Vec<Contact> = Vec::new();
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                // Skip static-static pairs
                if !bodies[i].has_rb && !bodies[j].has_rb { continue; }

                let contact = match (bodies[i].shape, bodies[j].shape) {
                    (ColliderShape::Box, ColliderShape::Box) => {
                        aabb_contact(
                            bodies[i].world_min, bodies[i].world_max,
                            bodies[j].world_min, bodies[j].world_max,
                            bodies[i].eid, bodies[j].eid,
                        )
                    }
                    (ColliderShape::Sphere, ColliderShape::Sphere) => {
                        sphere_sphere_contact(
                            bodies[i].world_center, bodies[i].world_radius,
                            bodies[j].world_center, bodies[j].world_radius,
                            bodies[i].eid, bodies[j].eid,
                        )
                    }
                    (ColliderShape::Sphere, ColliderShape::Box) => {
                        sphere_aabb_contact(
                            bodies[i].world_center, bodies[i].world_radius,
                            bodies[j].world_min, bodies[j].world_max,
                            bodies[i].eid, bodies[j].eid,
                        )
                    }
                    (ColliderShape::Box, ColliderShape::Sphere) => {
                        sphere_aabb_contact(
                            bodies[j].world_center, bodies[j].world_radius,
                            bodies[i].world_min, bodies[i].world_max,
                            bodies[j].eid, bodies[i].eid,
                        ).map(|mut c| {
                            std::mem::swap(&mut c.entity_a, &mut c.entity_b);
                            c.normal = -c.normal;
                            c
                        })
                    }
                };
                if let Some(c) = contact {
                    contacts.push(c);
                }
            }
        }

        // 4. Resolve contacts
        for contact in &contacts {
            let Some(a_idx) = bodies.iter().position(|b| b.eid == contact.entity_a) else { continue; };
            let Some(b_idx) = bodies.iter().position(|b| b.eid == contact.entity_b) else { continue; };

            // Skip if either is trigger
            if bodies[a_idx].is_trigger || bodies[b_idx].is_trigger { continue; }

            let restitution = (bodies[a_idx].restitution + bodies[b_idx].restitution) * 0.5;
            let friction = (bodies[a_idx].friction + bodies[b_idx].friction) * 0.5;

            // Get masses (0 = static/infinite mass)
            let mass_a = world.get_rigid_body(contact.entity_a)
                .map(|rb| rb.mass).unwrap_or(0.0);
            let mass_b = world.get_rigid_body(contact.entity_b)
                .map(|rb| rb.mass).unwrap_or(0.0);

            // Impulse resolution
            let mut vel_a = world.get_rigid_body(contact.entity_a)
                .map(|rb| rb.velocity).unwrap_or(Vec3::ZERO);
            let mut vel_b = world.get_rigid_body(contact.entity_b)
                .map(|rb| rb.velocity).unwrap_or(Vec3::ZERO);

            resolve_impulse(&mut vel_a, mass_a, &mut vel_b, mass_b, contact.normal, restitution, friction);

            if let Some(rb) = world.get_rigid_body_mut(contact.entity_a) {
                rb.velocity = vel_a;
            }
            if let Some(rb) = world.get_rigid_body_mut(contact.entity_b) {
                rb.velocity = vel_b;
            }

            // Position correction
            let inv_mass_a = if mass_a > 0.0 { 1.0 / mass_a } else { 0.0 };
            let inv_mass_b = if mass_b > 0.0 { 1.0 / mass_b } else { 0.0 };

            let mut pos_a = world.get_transform(contact.entity_a)
                .map(|t| t.position).unwrap_or(Vec3::ZERO);
            let mut pos_b = world.get_transform(contact.entity_b)
                .map(|t| t.position).unwrap_or(Vec3::ZERO);

            correct_positions(
                &mut pos_a, inv_mass_a,
                &mut pos_b, inv_mass_b,
                contact.normal, contact.depth,
            );

            if let Some(t) = world.get_transform_mut(contact.entity_a) {
                t.position = pos_a;
            }
            if let Some(t) = world.get_transform_mut(contact.entity_b) {
                t.position = pos_b;
            }
        }

        // 5. Ground plane collision (for entities with RigidBody)
        for &eid in &entities {
            let Some(_rb) = world.get_rigid_body(eid) else { continue; };
            let Some(t) = world.get_transform(eid) else { continue; };
            if t.position.y <= GROUND_Y {
                let restitution = world.get_collider(eid)
                    .map(|c| c.restitution).unwrap_or(0.0);
                if let Some(t) = world.get_transform_mut(eid) {
                    t.position.y = GROUND_Y;
                }
                let friction = world.get_collider(eid)
                    .map(|c| c.friction).unwrap_or(0.5);
                if let Some(rb) = world.get_rigid_body_mut(eid) {
                    if rb.velocity.y < 0.0 {
                        rb.velocity.y = -rb.velocity.y * restitution;
                        if rb.velocity.y.abs() < 0.1 {
                            rb.velocity.y = 0.0;
                        }
                    }
                    // Ground friction: dampen horizontal velocity
                    if rb.velocity.y.abs() < 0.1 {
                        let damp = (1.0 - friction * dt * 5.0).max(0.0);
                        rb.velocity.x *= damp;
                        rb.velocity.z *= damp;
                    }
                }
            }
        }

        // 6. Generate events
        collision_state.update(&contacts)
    }
}
