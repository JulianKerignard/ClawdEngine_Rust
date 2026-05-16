// Raycasting is exposed to user scripts via ScriptContext; several fields and
// helpers are part of the public API even though demo scripts don't exercise
// them all yet.
#![allow(dead_code)]

use glam::Vec3;
use crate::core::{ColliderShape, EntityId, World};

/// Result of a raycast hit.
#[derive(Clone, Debug)]
pub struct RayHit {
    pub entity: EntityId,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Ray-AABB intersection using slab method. Returns Some(t) if hit, t >= 0.
pub fn ray_aabb(origin: Vec3, dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Option<f32> {
    let inv_dir = Vec3::new(1.0 / dir.x, 1.0 / dir.y, 1.0 / dir.z);
    let t1 = (aabb_min - origin) * inv_dir;
    let t2 = (aabb_max - origin) * inv_dir;
    let t_min_v = t1.min(t2);
    let t_max_v = t1.max(t2);
    let t_enter = t_min_v.x.max(t_min_v.y).max(t_min_v.z);
    let t_exit = t_max_v.x.min(t_max_v.y).min(t_max_v.z);
    if t_enter > t_exit || t_exit < 0.0 {
        None
    } else {
        Some(t_enter.max(0.0))
    }
}

/// Approximate AABB surface normal at a hit point (nearest face).
fn aabb_normal_at(point: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Vec3 {
    let center = (aabb_min + aabb_max) * 0.5;
    let half = (aabb_max - aabb_min) * 0.5;
    let local = point - center;
    let d = Vec3::new(
        if half.x > 1e-6 { (local.x / half.x).abs() } else { 0.0 },
        if half.y > 1e-6 { (local.y / half.y).abs() } else { 0.0 },
        if half.z > 1e-6 { (local.z / half.z).abs() } else { 0.0 },
    );
    if d.x > d.y && d.x > d.z {
        Vec3::new(local.x.signum(), 0.0, 0.0)
    } else if d.y > d.z {
        Vec3::new(0.0, local.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, local.z.signum())
    }
}

/// Ray-sphere intersection. Returns Some(t) if hit, t >= 0.
pub fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let a = dir.dot(dir);
    let b = 2.0 * oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    if t1 >= 0.0 {
        Some(t1)
    } else if t2 >= 0.0 {
        Some(t2)
    } else {
        None
    }
}

/// Cast a ray against all colliders in the world. Returns the closest hit.
pub fn raycast(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RayHit> {
    let dir = direction.normalize();
    let mut closest: Option<RayHit> = None;

    for eid in world.iter_entities() {
        let collider = match world.get_collider(eid) {
            Some(c) => *c,
            None => continue,
        };
        let transform = match world.get_transform(eid) {
            Some(t) => *t,
            None => continue,
        };
        let wt = world.get_world_transform(eid).unwrap_or(transform);
        let (aabb_min, aabb_max) = collider.world_aabb(wt.position, wt.rotation, wt.scale);

        let hit_t = match collider.shape {
            ColliderShape::Box => ray_aabb(origin, dir, aabb_min, aabb_max),
            ColliderShape::Sphere => {
                let world_center = (aabb_min + aabb_max) * 0.5;
                let max_scale = wt.scale.x.max(wt.scale.y).max(wt.scale.z);
                let world_radius = collider.radius * max_scale;
                ray_sphere(origin, dir, world_center, world_radius)
            }
        };

        if let Some(t) = hit_t {
            if t <= max_distance {
                let point = origin + dir * t;
                let normal = match collider.shape {
                    ColliderShape::Box => aabb_normal_at(point, aabb_min, aabb_max),
                    ColliderShape::Sphere => {
                        let center = (aabb_min + aabb_max) * 0.5;
                        (point - center).normalize_or_zero()
                    }
                };
                if closest.as_ref().map_or(true, |c| t < c.distance) {
                    closest = Some(RayHit { entity: eid, point, normal, distance: t });
                }
            }
        }
    }

    closest
}

/// Cast a ray and return ALL hits sorted by distance.
pub fn raycast_all(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Vec<RayHit> {
    let dir = direction.normalize();
    let mut hits = Vec::new();

    for eid in world.iter_entities() {
        let collider = match world.get_collider(eid) {
            Some(c) => *c,
            None => continue,
        };
        let transform = match world.get_transform(eid) {
            Some(t) => *t,
            None => continue,
        };
        let wt = world.get_world_transform(eid).unwrap_or(transform);
        let (aabb_min, aabb_max) = collider.world_aabb(wt.position, wt.rotation, wt.scale);

        let hit_t = match collider.shape {
            ColliderShape::Box => ray_aabb(origin, dir, aabb_min, aabb_max),
            ColliderShape::Sphere => {
                let world_center = (aabb_min + aabb_max) * 0.5;
                let max_scale = wt.scale.x.max(wt.scale.y).max(wt.scale.z);
                let world_radius = collider.radius * max_scale;
                ray_sphere(origin, dir, world_center, world_radius)
            }
        };

        if let Some(t) = hit_t {
            if t <= max_distance {
                let point = origin + dir * t;
                let normal = match collider.shape {
                    ColliderShape::Box => aabb_normal_at(point, aabb_min, aabb_max),
                    ColliderShape::Sphere => {
                        let center = (aabb_min + aabb_max) * 0.5;
                        (point - center).normalize_or_zero()
                    }
                };
                hits.push(RayHit { entity: eid, point, normal, distance: t });
            }
        }
    }

    hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    hits
}
