use glam::Vec3;
use crate::core::{ColliderShape, EntityId, World};
use crate::renderer::mesh::MeshStore;

/// Result of a raycast hit.
#[derive(Clone, Debug)]
#[allow(dead_code)]
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

/// Ray-capsule intersection. Tests cylinder body + two hemisphere endcaps.
pub fn ray_capsule(origin: Vec3, dir: Vec3, cap_a: Vec3, cap_b: Vec3, radius: f32) -> Option<f32> {
    let t_a = ray_sphere(origin, dir, cap_a, radius);
    let t_b = ray_sphere(origin, dir, cap_b, radius);

    let ab = cap_b - cap_a;
    let ab_len_sq = ab.length_squared();
    let t_cyl = if ab_len_sq > 1e-12 {
        let ab_n = ab * (1.0 / ab_len_sq.sqrt());
        let ao = origin - cap_a;
        let dir_perp = dir - ab_n * dir.dot(ab_n);
        let ao_perp = ao - ab_n * ao.dot(ab_n);

        let a = dir_perp.dot(dir_perp);
        let b = 2.0 * ao_perp.dot(dir_perp);
        let c = ao_perp.dot(ao_perp) - radius * radius;
        let disc = b * b - 4.0 * a * c;

        if disc >= 0.0 && a > 1e-12 {
            let sqrt_d = disc.sqrt();
            let t1 = (-b - sqrt_d) / (2.0 * a);
            let t2 = (-b + sqrt_d) / (2.0 * a);
            let t = if t1 >= 0.0 { t1 } else { t2 };
            if t >= 0.0 {
                let hit = origin + dir * t;
                let proj = (hit - cap_a).dot(ab_n);
                if proj >= 0.0 && proj <= ab_len_sq.sqrt() {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    [t_a, t_b, t_cyl].iter().filter_map(|t| *t).reduce(f32::min)
}

/// Capsule surface normal at a hit point.
fn capsule_normal_at(point: Vec3, cap_a: Vec3, cap_b: Vec3) -> Vec3 {
    let ab = cap_b - cap_a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return (point - cap_a).normalize_or_zero();
    }
    let t = ((point - cap_a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = cap_a + ab * t;
    (point - closest).normalize_or_zero()
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
            ColliderShape::Capsule => {
                let (ca, cb, cr) = collider.capsule_segment(wt.position, wt.rotation, wt.scale);
                ray_capsule(origin, dir, ca, cb, cr)
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
                    ColliderShape::Capsule => {
                        let (ca, cb, _) = collider.capsule_segment(wt.position, wt.rotation, wt.scale);
                        capsule_normal_at(point, ca, cb)
                    }
                };
                if closest.as_ref().is_none_or(|c| t < c.distance) {
                    closest = Some(RayHit { entity: eid, point, normal, distance: t });
                }
            }
        }
    }

    closest
}

/// Cast a ray and return ALL hits sorted by distance.
#[allow(dead_code)]
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
            ColliderShape::Capsule => {
                let (ca, cb, cr) = collider.capsule_segment(wt.position, wt.rotation, wt.scale);
                ray_capsule(origin, dir, ca, cb, cr)
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
                    ColliderShape::Capsule => {
                        let (ca, cb, _) = collider.capsule_segment(wt.position, wt.rotation, wt.scale);
                        capsule_normal_at(point, ca, cb)
                    }
                };
                hits.push(RayHit { entity: eid, point, normal, distance: t });
            }
        }
    }

    hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    hits
}

/// Cast a ray against the world-space AABBs of all visible MeshRenderer entities.
/// This catches entities **without** a Collider component.
/// Returns the closest hit.
pub fn raycast_mesh_aabb(
    world: &World,
    mesh_store: &MeshStore,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RayHit> {
    let dir = direction.normalize();
    let mut closest: Option<RayHit> = None;

    for eid in world.iter_entities() {
        let mr = match world.get_mesh_renderer(eid) {
            Some(mr) if mr.visible => mr,
            _ => continue,
        };
        let mesh_id = match mr.mesh_id {
            Some(id) => id,
            None => continue,
        };
        let aabb = match mesh_store.get_aabb(mesh_id) {
            Some(a) => a,
            None => continue,
        };
        let transform = match world.get_transform(eid) {
            Some(t) => *t,
            None => continue,
        };
        let wt = world.get_world_transform(eid).unwrap_or(transform);
        let model = glam::Mat4::from_scale_rotation_translation(wt.scale, wt.rotation, wt.position);
        let (wmin, wmax) = aabb.transformed(model);

        if let Some(t) = ray_aabb(origin, dir, wmin, wmax) {
            if t <= max_distance {
                let point = origin + dir * t;
                let normal = aabb_normal_at(point, wmin, wmax);
                if closest.as_ref().is_none_or(|c| t < c.distance) {
                    closest = Some(RayHit { entity: eid, point, normal, distance: t });
                }
            }
        }
    }

    closest
}

/// Cast a ray against all visible MeshRenderer AABBs. Returns all hits sorted by distance.
pub fn raycast_mesh_aabb_all(
    world: &World,
    mesh_store: &MeshStore,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Vec<RayHit> {
    let dir = direction.normalize();
    let mut hits = Vec::new();

    for eid in world.iter_entities() {
        let mr = match world.get_mesh_renderer(eid) {
            Some(mr) if mr.visible => mr,
            _ => continue,
        };
        let mesh_id = match mr.mesh_id {
            Some(id) => id,
            None => continue,
        };
        let aabb = match mesh_store.get_aabb(mesh_id) {
            Some(a) => a,
            None => continue,
        };
        let transform = match world.get_transform(eid) {
            Some(t) => *t,
            None => continue,
        };
        let wt = world.get_world_transform(eid).unwrap_or(transform);
        let model = glam::Mat4::from_scale_rotation_translation(wt.scale, wt.rotation, wt.position);
        let (wmin, wmax) = aabb.transformed(model);

        if let Some(t) = ray_aabb(origin, dir, wmin, wmax) {
            if t <= max_distance {
                let point = origin + dir * t;
                let normal = aabb_normal_at(point, wmin, wmax);
                hits.push(RayHit { entity: eid, point, normal, distance: t });
            }
        }
    }

    hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    hits
}
