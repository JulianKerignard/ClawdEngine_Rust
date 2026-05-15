// CollisionEvent is consumed by user scripts via ScriptContext::collisions().
// Some fields are exposed for API completeness even when demo scripts don't
// inspect them.
#![allow(dead_code)]

use std::collections::HashSet;
use glam::Vec3;
use crate::core::EntityId;

// ---- Contact ----

pub struct Contact {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    pub normal: Vec3,
    pub depth: f32,
}

// ---- AABB-AABB overlap + MTV ----

pub fn aabb_contact(
    min_a: Vec3, max_a: Vec3,
    min_b: Vec3, max_b: Vec3,
    id_a: EntityId, id_b: EntityId,
) -> Option<Contact> {
    let overlap_x = max_a.x.min(max_b.x) - min_a.x.max(min_b.x);
    let overlap_y = max_a.y.min(max_b.y) - min_a.y.max(min_b.y);
    let overlap_z = max_a.z.min(max_b.z) - min_a.z.max(min_b.z);

    if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    let center_a = (min_a + max_a) * 0.5;
    let center_b = (min_b + max_b) * 0.5;

    let (depth, normal) = if overlap_x <= overlap_y && overlap_x <= overlap_z {
        let sign = if center_b.x > center_a.x { 1.0 } else { -1.0 };
        (overlap_x, Vec3::new(sign, 0.0, 0.0))
    } else if overlap_y <= overlap_z {
        let sign = if center_b.y > center_a.y { 1.0 } else { -1.0 };
        (overlap_y, Vec3::new(0.0, sign, 0.0))
    } else {
        let sign = if center_b.z > center_a.z { 1.0 } else { -1.0 };
        (overlap_z, Vec3::new(0.0, 0.0, sign))
    };

    Some(Contact { entity_a: id_a, entity_b: id_b, normal, depth })
}

// ---- Sphere-Sphere contact ----

pub fn sphere_sphere_contact(
    center_a: Vec3, radius_a: f32,
    center_b: Vec3, radius_b: f32,
    id_a: EntityId, id_b: EntityId,
) -> Option<Contact> {
    let d = center_b - center_a;
    let dist_sq = d.length_squared();
    let sum_radii = radius_a + radius_b;
    if dist_sq >= sum_radii * sum_radii { return None; }

    let dist = dist_sq.sqrt();
    let normal = if dist > 1e-6 { d / dist } else { Vec3::Y };
    let depth = sum_radii - dist;

    Some(Contact { entity_a: id_a, entity_b: id_b, normal, depth })
}

// ---- Sphere-AABB contact ----

pub fn sphere_aabb_contact(
    sphere_center: Vec3, radius: f32,
    aabb_min: Vec3, aabb_max: Vec3,
    id_sphere: EntityId, id_aabb: EntityId,
) -> Option<Contact> {
    let closest = sphere_center.clamp(aabb_min, aabb_max);
    let diff = sphere_center - closest;
    let dist_sq = diff.length_squared();

    if dist_sq > 0.0 && dist_sq < radius * radius {
        // Sphere center outside AABB but within radius
        let dist = dist_sq.sqrt();
        let normal = diff / dist;
        Some(Contact {
            entity_a: id_sphere, entity_b: id_aabb,
            normal, depth: radius - dist,
        })
    } else if dist_sq <= 0.0 {
        // Sphere center inside AABB — find nearest face (MTV)
        let aabb_center = (aabb_min + aabb_max) * 0.5;
        let aabb_he = (aabb_max - aabb_min) * 0.5;
        let local = sphere_center - aabb_center;
        let dx = aabb_he.x - local.x.abs();
        let dy = aabb_he.y - local.y.abs();
        let dz = aabb_he.z - local.z.abs();
        let (depth, mut normal) = if dx <= dy && dx <= dz {
            (dx + radius, Vec3::X)
        } else if dy <= dz {
            (dy + radius, Vec3::Y)
        } else {
            (dz + radius, Vec3::Z)
        };
        if local.dot(normal) < 0.0 { normal = -normal; }
        Some(Contact {
            entity_a: id_sphere, entity_b: id_aabb,
            normal, depth,
        })
    } else {
        None
    }
}

// ---- Impulse resolution ----

pub fn resolve_impulse(
    vel_a: &mut Vec3, mass_a: f32,
    vel_b: &mut Vec3, mass_b: f32,
    normal: Vec3,
    restitution: f32,
    friction: f32,
) {
    let inv_mass_a = if mass_a > 0.0 { 1.0 / mass_a } else { 0.0 };
    let inv_mass_b = if mass_b > 0.0 { 1.0 / mass_b } else { 0.0 };
    let inv_mass_sum = inv_mass_a + inv_mass_b;
    if inv_mass_sum == 0.0 { return; }

    let rel_vel = *vel_a - *vel_b;
    let vel_along_normal = rel_vel.dot(normal);

    if vel_along_normal > 0.0 { return; }

    let j = -(1.0 + restitution) * vel_along_normal / inv_mass_sum;

    *vel_a += inv_mass_a * j * normal;
    *vel_b -= inv_mass_b * j * normal;

    // Coulomb friction
    if friction > 0.0 {
        let rel_vel_after = *vel_a - *vel_b;
        let tangent_vel = rel_vel_after - rel_vel_after.dot(normal) * normal;
        let tangent_speed = tangent_vel.length();
        if tangent_speed > 1e-6 {
            let tangent = tangent_vel / tangent_speed;
            let jt = -tangent_speed / inv_mass_sum;
            let friction_impulse = if jt.abs() < j * friction {
                jt
            } else {
                -j * friction * jt.signum()
            };
            *vel_a += inv_mass_a * friction_impulse * tangent;
            *vel_b -= inv_mass_b * friction_impulse * tangent;
        }
    }
}

// ---- Position correction (anti-sinking) ----

const CORRECTION_PERCENT: f32 = 0.8;
const CORRECTION_SLOP: f32 = 0.01;

pub fn correct_positions(
    pos_a: &mut Vec3, inv_mass_a: f32,
    pos_b: &mut Vec3, inv_mass_b: f32,
    normal: Vec3, depth: f32,
) {
    let inv_sum = inv_mass_a + inv_mass_b;
    if inv_sum == 0.0 { return; }
    let correction = (depth - CORRECTION_SLOP).max(0.0) * CORRECTION_PERCENT / inv_sum;
    *pos_a -= inv_mass_a * correction * normal;
    *pos_b += inv_mass_b * correction * normal;
}

// ---- Collision Events ----

#[derive(Clone, Debug)]
pub enum CollisionEventKind {
    Enter,
    Stay,
    Exit,
}

#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub entity: EntityId,
    pub other: EntityId,
    pub kind: CollisionEventKind,
    pub normal: Vec3,
}

fn canonical_pair(a: EntityId, b: EntityId) -> (u32, u32) {
    let ai = a.index;
    let bi = b.index;
    if ai <= bi { (ai, bi) } else { (bi, ai) }
}

pub struct CollisionState {
    active_pairs: HashSet<(u32, u32)>,
}

impl CollisionState {
    pub fn new() -> Self {
        Self { active_pairs: HashSet::new() }
    }

    pub fn update(&mut self, contacts: &[Contact]) -> Vec<CollisionEvent> {
        let mut events = Vec::new();
        let mut current_pairs = HashSet::new();

        for c in contacts {
            let pair = canonical_pair(c.entity_a, c.entity_b);
            current_pairs.insert(pair);

            let kind = if self.active_pairs.contains(&pair) {
                CollisionEventKind::Stay
            } else {
                CollisionEventKind::Enter
            };

            events.push(CollisionEvent {
                entity: c.entity_a,
                other: c.entity_b,
                kind: kind.clone(),
                normal: c.normal,
            });
            events.push(CollisionEvent {
                entity: c.entity_b,
                other: c.entity_a,
                kind,
                normal: -c.normal,
            });
        }

        // Exit events
        for &pair in &self.active_pairs {
            if !current_pairs.contains(&pair) {
                let a = EntityId::new(pair.0, 0); // generation doesn't matter for events
                let b = EntityId::new(pair.1, 0);
                events.push(CollisionEvent {
                    entity: a, other: b,
                    kind: CollisionEventKind::Exit,
                    normal: Vec3::ZERO,
                });
                events.push(CollisionEvent {
                    entity: b, other: a,
                    kind: CollisionEventKind::Exit,
                    normal: Vec3::ZERO,
                });
            }
        }

        self.active_pairs = current_pairs;
        events
    }

    pub fn clear(&mut self) {
        self.active_pairs.clear();
    }
}
