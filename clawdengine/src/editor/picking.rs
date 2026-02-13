use glam::{Mat4, Vec3};

use crate::core::{EntityId, World};
use crate::editor::context::GizmoAxis;
use crate::renderer::camera::Camera;
use crate::renderer::mesh::MeshStore;

/// Ray-AABB intersection using the slab algorithm.
/// Returns Some(t) with t >= 0 if the ray hits, None otherwise.
fn ray_aabb_intersection(origin: Vec3, dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Option<f32> {
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

/// Pick the closest entity under the mouse cursor.
/// `mx`, `my` are viewport-relative coordinates (0,0 = top-left of viewport).
pub fn pick_entity(
    camera: &Camera,
    vp_w: f32,
    vp_h: f32,
    mx: f32,
    my: f32,
    world: &World,
    mesh_store: &MeshStore,
) -> Option<EntityId> {
    let (ray_origin, ray_dir) = camera.screen_to_ray(mx, my, vp_w, vp_h);

    if ray_dir.length_squared() < 1e-10 {
        return None;
    }

    let mut closest: Option<(EntityId, f32)> = None;

    for eid in world.iter_entities() {
        let transform = match world.get_transform(eid) {
            Some(t) => t,
            None => continue,
        };
        let mesh_renderer = match world.get_mesh_renderer(eid) {
            Some(mr) => mr,
            None => continue,
        };
        if !mesh_renderer.visible {
            continue;
        }
        let mesh_id = match mesh_renderer.mesh_id {
            Some(id) => id,
            None => continue,
        };
        let aabb = match mesh_store.get_aabb(mesh_id) {
            Some(a) => a,
            None => continue,
        };

        let wt = world.get_world_transform(eid).unwrap_or(*transform);
        let model_matrix = Mat4::from_scale_rotation_translation(
            wt.scale,
            wt.rotation,
            wt.position,
        );
        let (world_min, world_max) = aabb.transformed(model_matrix);

        if let Some(t) = ray_aabb_intersection(ray_origin, ray_dir, world_min, world_max) {
            match &closest {
                Some((_, best_t)) if t >= *best_t => {}
                _ => closest = Some((eid, t)),
            }
        }
    }

    closest.map(|(eid, _)| eid)
}

/// Closest distance between two 3D lines.
/// Returns (distance, parameter_on_line_b).
fn ray_line_closest(
    ray_o: Vec3,
    ray_d: Vec3,
    line_o: Vec3,
    line_d: Vec3,
) -> (f32, f32) {
    let w0 = ray_o - line_o;
    let a = ray_d.dot(ray_d);
    let b = ray_d.dot(line_d);
    let c = line_d.dot(line_d);
    let d = ray_d.dot(w0);
    let e = line_d.dot(w0);

    let denom = a * c - b * b;
    if denom.abs() < 1e-10 {
        // Parallel lines
        let t_line = -e / c.max(1e-10);
        let closest_on_ray = ray_o;
        let closest_on_line = line_o + line_d * t_line;
        return ((closest_on_ray - closest_on_line).length(), t_line);
    }

    let s = (b * e - c * d) / denom;
    let t = (a * e - b * d) / denom;

    let p1 = ray_o + ray_d * s;
    let p2 = line_o + line_d * t;

    ((p1 - p2).length(), t)
}

/// Pick a gizmo axis under the cursor.
/// Returns Some((axis, t_parameter)) if an axis is close enough.
pub fn pick_gizmo_axis(
    camera: &Camera,
    vp_w: f32,
    vp_h: f32,
    mx: f32,
    my: f32,
    gizmo_origin: Vec3,
) -> Option<(GizmoAxis, f32)> {
    let (ray_o, ray_d) = camera.screen_to_ray(mx, my, vp_w, vp_h);
    if ray_d.length_squared() < 1e-10 {
        return None;
    }

    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];

    // Gizmo shaft length (must match render)
    let shaft_len = 1.6_f32;
    let threshold = 0.15_f32;

    let mut best: Option<(GizmoAxis, f32, f32)> = None; // (axis, t, distance)

    for (axis, dir) in &axes {
        let (dist, t) = ray_line_closest(ray_o, ray_d, gizmo_origin, *dir);

        // Only pick if t is within the shaft range and close enough
        if t >= -0.1 && t <= shaft_len && dist < threshold {
            match &best {
                Some((_, _, best_dist)) if dist >= *best_dist => {}
                _ => best = Some((*axis, t, dist)),
            }
        }
    }

    best.map(|(axis, t, _)| (axis, t))
}

/// Get two perpendicular axes to a given normal.
fn perpendicular_axes(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = normal.cross(up).normalize();
    let v = normal.cross(u).normalize();
    (u, v)
}

/// Pick the closest rotation ring under the cursor.
/// Returns Some((axis, initial_angle)) if a ring is close enough.
pub fn pick_gizmo_ring(
    camera: &Camera,
    vp_w: f32,
    vp_h: f32,
    mx: f32,
    my: f32,
    gizmo_origin: Vec3,
) -> Option<(GizmoAxis, f32)> {
    let (ray_o, ray_d) = camera.screen_to_ray(mx, my, vp_w, vp_h);
    if ray_d.length_squared() < 1e-10 {
        return None;
    }

    let ring_radius = 1.2_f32;
    let threshold = 0.2_f32;

    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];

    let mut best: Option<(GizmoAxis, f32, f32)> = None; // (axis, angle, dist_from_ring)

    for (axis, normal) in &axes {
        let denom = ray_d.dot(*normal);
        if denom.abs() < 1e-6 {
            continue; // ray parallel to ring plane
        }
        let t = (gizmo_origin - ray_o).dot(*normal) / denom;
        if t < 0.0 {
            continue; // behind camera
        }
        let hit = ray_o + ray_d * t;
        let offset = hit - gizmo_origin;
        let dist_from_center = offset.length();
        let dist_from_ring = (dist_from_center - ring_radius).abs();

        if dist_from_ring < threshold {
            let (u_axis, v_axis) = perpendicular_axes(*normal);
            let angle = offset.dot(v_axis).atan2(offset.dot(u_axis));
            match &best {
                Some((_, _, best_d)) if dist_from_ring >= *best_d => {}
                _ => best = Some((*axis, angle, dist_from_ring)),
            }
        }
    }

    best.map(|(axis, angle, _)| (axis, angle))
}

/// Project the current mouse ray onto a rotation ring plane, returning the angle.
pub fn project_ray_onto_ring(
    camera: &Camera,
    vp_w: f32,
    vp_h: f32,
    mx: f32,
    my: f32,
    origin: Vec3,
    axis: GizmoAxis,
) -> f32 {
    let (ray_o, ray_d) = camera.screen_to_ray(mx, my, vp_w, vp_h);
    let normal = axis.direction();
    let denom = ray_d.dot(normal);
    if denom.abs() < 1e-6 {
        return 0.0;
    }
    let t = (origin - ray_o).dot(normal) / denom;
    let hit = ray_o + ray_d * t;
    let offset = hit - origin;
    let (u_axis, v_axis) = perpendicular_axes(normal);
    offset.dot(v_axis).atan2(offset.dot(u_axis))
}

/// Project the current mouse ray onto an axis, returning the t parameter.
pub fn project_ray_onto_axis(
    camera: &Camera,
    vp_w: f32,
    vp_h: f32,
    mx: f32,
    my: f32,
    origin: Vec3,
    axis: GizmoAxis,
) -> f32 {
    let (ray_o, ray_d) = camera.screen_to_ray(mx, my, vp_w, vp_h);
    let (_, t) = ray_line_closest(ray_o, ray_d, origin, axis.direction());
    t
}
