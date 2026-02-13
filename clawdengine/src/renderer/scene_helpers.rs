use crate::core::{EntityId, LightKind, World};
use crate::editor::context::{EditorTool, GizmoAxis};
use super::gpu_context::SceneRenderer;
use super::line_pipeline::LineBatch;
use super::pipeline::{LightData, LightsUniforms};
use super::shadow::ShadowMap;

// ---- Constants ----

pub const AMBIENT_COLOR: [f32; 4] = [0.12, 0.14, 0.18, 1.0];
pub const GIZMO_SHAFT_LEN: f32 = 1.2;
pub const GIZMO_TIP_LEN: f32 = 1.6;
pub const GIZMO_LINE_WIDTH: f32 = 0.035;
pub const GIZMO_CONE_RADIUS: f32 = 0.07;
pub const GIZMO_CONE_SEGMENTS: u32 = 12;
pub const GIZMO_RING_RADIUS: f32 = 1.2;
pub const GIZMO_RING_WIDTH: f32 = 0.025;
pub const GIZMO_RING_SEGMENTS: u32 = 64;
pub const GIZMO_BOX_HALF: f32 = 0.06;
pub const LIGHT_ICON_COLOR: [f32; 4] = [0.94, 0.75, 0.25, 1.0];
pub const LIGHT_ICON_SIZE: f32 = 0.3;
pub const LIGHT_ICON_WIDTH: f32 = 0.02;
pub const GRID_HALF_SIZE: i32 = 10;
pub const GRID_COLOR: [f32; 4] = [0.4, 0.4, 0.4, 0.5];
pub const SELECTION_TINT: [f32; 4] = [0.294, 0.545, 0.745, 1.0];

const MAX_LIGHTS: usize = 4;

// ---- Light uniform building ----

pub fn build_light_uniforms(world: &World, queue: &wgpu::Queue, scene: &SceneRenderer) {
    let mut lights_data = [LightData {
        position: [0.0; 4],
        color: [0.0; 4],
        direction: [0.0; 4],
        spot_params: [0.0; 4],
    }; MAX_LIGHTS];
    let mut light_count = 0u32;

    for (eid, light) in world.lights_iter() {
        if light_count as usize >= MAX_LIGHTS {
            break;
        }
        let wt = world.get_world_transform(eid);
        let pos = wt.map(|t| t.position).unwrap_or(glam::Vec3::ZERO);
        let rot = wt.map(|t| t.rotation).unwrap_or(glam::Quat::IDENTITY);
        let i = light_count as usize;

        lights_data[i] = match light.kind {
            LightKind::Directional => {
                let dir = rot * glam::Vec3::new(0.0, -1.0, 0.0);
                LightData {
                    position: [0.0, 0.0, 0.0, 0.0],
                    color: [light.color.x, light.color.y, light.color.z, light.intensity],
                    direction: [dir.x, dir.y, dir.z, 0.0],
                    spot_params: [0.0; 4],
                }
            }
            LightKind::Point => LightData {
                position: [pos.x, pos.y, pos.z, 1.0],
                color: [light.color.x, light.color.y, light.color.z, light.intensity],
                direction: [0.0, 0.0, 0.0, light.range],
                spot_params: [0.0; 4],
            },
            LightKind::Spot => {
                let spot_dir = rot * glam::Vec3::new(0.0, -1.0, 0.0);
                LightData {
                    position: [pos.x, pos.y, pos.z, 2.0],
                    color: [light.color.x, light.color.y, light.color.z, light.intensity],
                    direction: [spot_dir.x, spot_dir.y, spot_dir.z, light.range],
                    spot_params: [light.inner_angle.cos(), light.outer_angle.cos(), 0.0, 0.0],
                }
            }
        };
        light_count += 1;
    }

    let lights_uniforms = LightsUniforms {
        ambient: AMBIENT_COLOR,
        count: light_count,
        _pad: [0.0; 3],
        lights: lights_data,
    };
    queue.write_buffer(
        &scene.lights_buffer,
        0,
        bytemuck::cast_slice(&[lights_uniforms]),
    );
}

pub fn update_shadow_vp(world: &World, queue: &wgpu::Queue, scene: &SceneRenderer) {
    let mut shadow_light_dir = glam::Vec3::new(0.0, -1.0, 0.0);
    for (eid, light) in world.lights_iter() {
        if light.kind == LightKind::Directional {
            let rot = world
                .get_world_transform(eid)
                .map(|t| t.rotation)
                .unwrap_or(glam::Quat::IDENTITY);
            shadow_light_dir = rot * glam::Vec3::new(0.0, -1.0, 0.0);
            break;
        }
    }
    let light_vp = ShadowMap::compute_light_vp(shadow_light_dir);
    scene.shadow_map.update_light_vp(queue, light_vp);
}

// ---- Gizmo drawing ----

pub fn draw_gizmos(
    line_batch: &mut LineBatch,
    gizmo_position: Option<glam::Vec3>,
    active_tool: EditorTool,
    gizmo_drag_axis: Option<GizmoAxis>,
    gizmo_hover_axis: Option<GizmoAxis>,
    eye: [f32; 3],
) {
    let pos = match gizmo_position {
        Some(v) => v,
        None => return,
    };
    let p = [pos.x, pos.y, pos.z];

    let highlight = gizmo_drag_axis.or(gizmo_hover_axis);
    let dragging = gizmo_drag_axis.is_some();

    let axis_color = |axis: GizmoAxis| -> [f32; 4] {
        let is_active = highlight == Some(axis);
        match axis {
            GizmoAxis::X => {
                if is_active { [1.0, 0.4, 0.4, 1.0] }
                else { [1.0, 0.2, 0.2, if dragging { 0.15 } else { 0.7 }] }
            }
            GizmoAxis::Y => {
                if is_active { [0.4, 1.0, 0.4, 1.0] }
                else { [0.2, 1.0, 0.2, if dragging { 0.15 } else { 0.7 }] }
            }
            GizmoAxis::Z => {
                if is_active { [0.5, 0.7, 1.0, 1.0] }
                else { [0.3, 0.5, 1.0, if dragging { 0.15 } else { 0.7 }] }
            }
        }
    };

    let should_draw = |axis: GizmoAxis| -> bool {
        !dragging || gizmo_drag_axis == Some(axis)
    };

    let axes = [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z];
    let dirs: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    match active_tool {
        EditorTool::Move => {
            for (axis, d) in axes.iter().zip(dirs.iter()) {
                if !should_draw(*axis) { continue; }
                let c = axis_color(*axis);
                let end = [
                    p[0] + d[0] * GIZMO_SHAFT_LEN,
                    p[1] + d[1] * GIZMO_SHAFT_LEN,
                    p[2] + d[2] * GIZMO_SHAFT_LEN,
                ];
                let tip = [
                    p[0] + d[0] * GIZMO_TIP_LEN,
                    p[1] + d[1] * GIZMO_TIP_LEN,
                    p[2] + d[2] * GIZMO_TIP_LEN,
                ];
                line_batch.push_billboard_line(p, end, c, GIZMO_LINE_WIDTH, eye);
                line_batch.push_cone(tip, end, GIZMO_CONE_RADIUS, c, GIZMO_CONE_SEGMENTS);
            }
        }
        EditorTool::Rotate => {
            for (axis, d) in axes.iter().zip(dirs.iter()) {
                if !should_draw(*axis) { continue; }
                let c = axis_color(*axis);
                let w = if highlight == Some(*axis) { GIZMO_RING_WIDTH * 2.0 } else { GIZMO_RING_WIDTH };
                line_batch.push_circle_arc(p, *d, GIZMO_RING_RADIUS, c, GIZMO_RING_SEGMENTS, w, eye);
            }
        }
        EditorTool::Scale => {
            for (axis, d) in axes.iter().zip(dirs.iter()) {
                if !should_draw(*axis) { continue; }
                let c = axis_color(*axis);
                let end = [
                    p[0] + d[0] * GIZMO_SHAFT_LEN,
                    p[1] + d[1] * GIZMO_SHAFT_LEN,
                    p[2] + d[2] * GIZMO_SHAFT_LEN,
                ];
                line_batch.push_billboard_line(p, end, c, GIZMO_LINE_WIDTH, eye);
                line_batch.push_box_marker(end, GIZMO_BOX_HALF, c, eye);
            }
        }
        EditorTool::Select => {}
    }
}

// ---- Light helper drawing ----

pub fn draw_light_helpers(
    line_batch: &mut LineBatch,
    world: &World,
    selected_entity: Option<EntityId>,
    cam_right: glam::Vec3,
    cam_up: glam::Vec3,
    eye: [f32; 3],
) {
    for (eid, light) in world.lights_iter() {
        let wt = match world.get_world_transform(eid) {
            Some(t) => t,
            None => continue,
        };
        let p = wt.position;
        let pos = [p.x, p.y, p.z];

        // Billboard star icon (always drawn)
        let r = cam_right * LIGHT_ICON_SIZE;
        let u = cam_up * LIGHT_ICON_SIZE;
        let w = LIGHT_ICON_WIDTH;

        // Horizontal branch
        line_batch.push_billboard_line(
            [pos[0] - r.x, pos[1] - r.y, pos[2] - r.z],
            [pos[0] + r.x, pos[1] + r.y, pos[2] + r.z],
            LIGHT_ICON_COLOR, w, eye,
        );
        // Vertical branch
        line_batch.push_billboard_line(
            [pos[0] - u.x, pos[1] - u.y, pos[2] - u.z],
            [pos[0] + u.x, pos[1] + u.y, pos[2] + u.z],
            LIGHT_ICON_COLOR, w, eye,
        );
        // Diagonal 1
        let d1 = (r + u) * 0.707;
        line_batch.push_billboard_line(
            [pos[0] - d1.x, pos[1] - d1.y, pos[2] - d1.z],
            [pos[0] + d1.x, pos[1] + d1.y, pos[2] + d1.z],
            LIGHT_ICON_COLOR, w, eye,
        );
        // Diagonal 2
        let d2 = (r - u) * 0.707;
        line_batch.push_billboard_line(
            [pos[0] - d2.x, pos[1] - d2.y, pos[2] - d2.z],
            [pos[0] + d2.x, pos[1] + d2.y, pos[2] + d2.z],
            LIGHT_ICON_COLOR, w, eye,
        );

        // Direction/range helpers only when selected
        if selected_entity != Some(eid) {
            continue;
        }
        let helper_color = [light.color.x, light.color.y, light.color.z, 0.5];

        match light.kind {
            LightKind::Directional => {
                draw_directional_helper(line_batch, &wt, helper_color, pos);
            }
            LightKind::Point => {
                draw_point_helper(line_batch, light, pos, eye);
            }
            LightKind::Spot => {
                draw_spot_helper(line_batch, &wt, light, pos);
            }
        }
    }
}

fn draw_directional_helper(
    line_batch: &mut LineBatch,
    transform: &crate::core::Transform,
    helper_color: [f32; 4],
    pos: [f32; 3],
) {
    let p = transform.position;
    let rot = transform.rotation;
    let dir = rot * glam::Vec3::new(0.0, -1.0, 0.0);
    let len = 3.0_f32;
    let end_center = p + dir * len;

    // Center ray
    line_batch.push_dashed_line(
        pos,
        [end_center.x, end_center.y, end_center.z],
        helper_color, 0.2, 0.15,
    );

    // Parallel rays offset by 0.5
    let perp = if dir.y.abs() > 0.9 {
        dir.cross(glam::Vec3::X).normalize()
    } else {
        dir.cross(glam::Vec3::Y).normalize()
    };
    let perp2 = dir.cross(perp).normalize();
    let offset = 0.5_f32;
    for &off in &[perp * offset, -perp * offset, perp2 * offset, -perp2 * offset] {
        let s = p + off;
        let e = s + dir * len;
        line_batch.push_dashed_line(
            [s.x, s.y, s.z],
            [e.x, e.y, e.z],
            helper_color, 0.2, 0.15,
        );
    }

    // Arrow tip at center
    let arrow_base = p + dir * (len - 0.3);
    line_batch.push_cone(
        [end_center.x, end_center.y, end_center.z],
        [arrow_base.x, arrow_base.y, arrow_base.z],
        0.1, helper_color, 8,
    );
}

fn draw_point_helper(
    line_batch: &mut LineBatch,
    light: &crate::core::Light,
    pos: [f32; 3],
    eye: [f32; 3],
) {
    let p = glam::Vec3::from(pos);
    let radius = light.range;
    let circle_color = [light.color.x, light.color.y, light.color.z, 0.3];
    let segs = 32;
    let cw = 0.01;

    line_batch.push_circle_arc(pos, [0.0, 1.0, 0.0], radius, circle_color, segs, cw, eye);
    line_batch.push_circle_arc(pos, [0.0, 0.0, 1.0], radius, circle_color, segs, cw, eye);
    line_batch.push_circle_arc(pos, [1.0, 0.0, 0.0], radius, circle_color, segs, cw, eye);

    let ray_color = [light.color.x, light.color.y, light.color.z, 0.25];
    for &dir in &[
        glam::Vec3::X, glam::Vec3::NEG_X,
        glam::Vec3::Y, glam::Vec3::NEG_Y,
        glam::Vec3::Z, glam::Vec3::NEG_Z,
    ] {
        let ray_end = p + dir * radius;
        line_batch.push_dashed_line(
            pos,
            [ray_end.x, ray_end.y, ray_end.z],
            ray_color, 0.15, 0.1,
        );
    }
}

fn draw_spot_helper(
    line_batch: &mut LineBatch,
    transform: &crate::core::Transform,
    light: &crate::core::Light,
    pos: [f32; 3],
) {
    let p = transform.position;
    let rot = transform.rotation;
    let spot_dir = rot * glam::Vec3::new(0.0, -1.0, 0.0);
    let cone_len = light.range;
    let cone_radius = cone_len * light.outer_angle.tan();
    let cone_center = p + spot_dir * cone_len;
    let cone_color = [light.color.x, light.color.y, light.color.z, 0.3];

    let perp1 = if spot_dir.y.abs() > 0.9 {
        spot_dir.cross(glam::Vec3::X).normalize()
    } else {
        spot_dir.cross(glam::Vec3::Y).normalize()
    };
    let perp2 = spot_dir.cross(perp1).normalize();

    let cone_segs = 16u32;
    let mut prev_base = glam::Vec3::ZERO;
    for seg in 0..=cone_segs {
        let angle = (seg as f32 / cone_segs as f32) * std::f32::consts::TAU;
        let base_pt = cone_center
            + perp1 * (angle.cos() * cone_radius)
            + perp2 * (angle.sin() * cone_radius);
        if seg > 0 {
            line_batch.push_line(
                [prev_base.x, prev_base.y, prev_base.z],
                [base_pt.x, base_pt.y, base_pt.z],
                cone_color,
            );
        }
        if seg % 4 == 0 && seg < cone_segs {
            line_batch.push_dashed_line(
                [p.x, p.y, p.z],
                [base_pt.x, base_pt.y, base_pt.z],
                cone_color, 0.2, 0.15,
            );
        }
        prev_base = base_pt;
    }

    // Center ray
    line_batch.push_dashed_line(
        pos,
        [cone_center.x, cone_center.y, cone_center.z],
        [light.color.x, light.color.y, light.color.z, 0.5],
        0.2, 0.15,
    );
}

// ---- Collider debug wireframe ----

const COLLIDER_COLOR: [f32; 4] = [0.2, 1.0, 0.3, 0.8];
const COLLIDER_SPHERE_SEGS: u32 = 32;

pub fn draw_collider_debug(
    line_batch: &mut LineBatch,
    world: &World,
    selected: &[EntityId],
) {
    for &eid in selected {
        let Some(collider) = world.get_collider(eid) else { continue; };
        let Some(wt) = world.get_world_transform(eid) else { continue; };

        let (wmin, wmax) = collider.world_aabb(wt.position, wt.rotation, wt.scale);

        match collider.shape {
            crate::core::ColliderShape::Box => {
                line_batch.push_aabb_wireframe(
                    [wmin.x, wmin.y, wmin.z],
                    [wmax.x, wmax.y, wmax.z],
                    COLLIDER_COLOR,
                );
            }
            crate::core::ColliderShape::Sphere => {
                let center = (wmin + wmax) * 0.5;
                let max_scale = wt.scale.x.max(wt.scale.y).max(wt.scale.z);
                let radius = collider.radius * max_scale;
                let c = [center.x, center.y, center.z];
                push_circle_thin(line_batch, c, [0.0, 0.0, 1.0], radius, COLLIDER_SPHERE_SEGS, COLLIDER_COLOR);
                push_circle_thin(line_batch, c, [0.0, 1.0, 0.0], radius, COLLIDER_SPHERE_SEGS, COLLIDER_COLOR);
                push_circle_thin(line_batch, c, [1.0, 0.0, 0.0], radius, COLLIDER_SPHERE_SEGS, COLLIDER_COLOR);
            }
        }
    }
}

fn push_circle_thin(
    line_batch: &mut LineBatch,
    center: [f32; 3],
    axis: [f32; 3],
    radius: f32,
    segments: u32,
    color: [f32; 4],
) {
    let ax = glam::Vec3::from(axis).normalize();
    let perp = if ax.y.abs() < 0.9 {
        ax.cross(glam::Vec3::Y).normalize()
    } else {
        ax.cross(glam::Vec3::X).normalize()
    };
    let perp2 = ax.cross(perp);
    let c = glam::Vec3::from(center);

    let mut prev = c + perp * radius;
    for i in 1..=segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let pt = c + (perp * angle.cos() + perp2 * angle.sin()) * radius;
        line_batch.push_line(
            [prev.x, prev.y, prev.z],
            [pt.x, pt.y, pt.z],
            color,
        );
        prev = pt;
    }
}

// ---- Camera helpers ----

const CAM_COLOR: [f32; 4] = [0.53, 0.86, 0.92, 0.8];
const CAM_COLOR_DIM: [f32; 4] = [0.53, 0.86, 0.92, 0.3];
const CAM_ICON_SIZE: f32 = 0.25;
const CAM_ICON_WIDTH: f32 = 0.02;

pub fn draw_camera_helpers(
    line_batch: &mut LineBatch,
    world: &World,
    selected_entity: Option<EntityId>,
    cam_right: glam::Vec3,
    cam_up: glam::Vec3,
    eye: [f32; 3],
) {
    for eid in world.iter_entities() {
        let Some(cam) = world.get_camera(eid) else { continue };
        let Some(wt) = world.get_world_transform(eid) else { continue };

        let color = if selected_entity == Some(eid) { CAM_COLOR } else { CAM_COLOR_DIM };

        // Billboard camera icon
        draw_camera_icon(line_batch, wt.position, cam_right, cam_up, color, eye);

        // Frustum wireframe
        draw_camera_frustum(line_batch, &wt, cam, color);
    }
}

fn draw_camera_icon(
    line_batch: &mut LineBatch,
    pos: glam::Vec3,
    cam_right: glam::Vec3,
    cam_up: glam::Vec3,
    color: [f32; 4],
    eye: [f32; 3],
) {
    let s = CAM_ICON_SIZE;
    let w = CAM_ICON_WIDTH;
    let r = cam_right;
    let u = cam_up;

    // Camera body: rectangle centered on pos
    let bl = pos - r * s * 0.7 - u * s * 0.5;
    let br = pos + r * s * 0.3 - u * s * 0.5;
    let tr = pos + r * s * 0.3 + u * s * 0.5;
    let tl = pos - r * s * 0.7 + u * s * 0.5;

    line_batch.push_billboard_line(bl.into(), br.into(), color, w, eye);
    line_batch.push_billboard_line(br.into(), tr.into(), color, w, eye);
    line_batch.push_billboard_line(tr.into(), tl.into(), color, w, eye);
    line_batch.push_billboard_line(tl.into(), bl.into(), color, w, eye);

    // Lens: triangle on the right side
    let lens_tip = pos + r * s * 0.8;
    let lens_top = pos + r * s * 0.3 + u * s * 0.3;
    let lens_bot = pos + r * s * 0.3 - u * s * 0.3;

    line_batch.push_billboard_line(lens_top.into(), lens_tip.into(), color, w, eye);
    line_batch.push_billboard_line(lens_tip.into(), lens_bot.into(), color, w, eye);
    line_batch.push_billboard_line(lens_bot.into(), lens_top.into(), color, w, eye);

    // Film reel circle on top-left
    let reel_center = pos - r * s * 0.4 + u * s * 0.7;
    let reel_r = s * 0.2;
    let segs = 8u32;
    let mut prev = reel_center + r * reel_r;
    for i in 1..=segs {
        let angle = (i as f32 / segs as f32) * std::f32::consts::TAU;
        let pt = reel_center + r * (reel_r * angle.cos()) + u * (reel_r * angle.sin());
        line_batch.push_billboard_line(prev.into(), pt.into(), color, w, eye);
        prev = pt;
    }
}

fn draw_camera_frustum(
    line_batch: &mut LineBatch,
    wt: &crate::core::Transform,
    cam: &crate::core::CameraComponent,
    color: [f32; 4],
) {
    let fwd = wt.rotation * glam::Vec3::new(0.0, 0.0, -1.0);
    let right = wt.rotation * glam::Vec3::new(1.0, 0.0, 0.0);
    let up = wt.rotation * glam::Vec3::new(0.0, 1.0, 0.0);
    let pos = wt.position;

    let aspect = 16.0 / 9.0;
    let near_d = cam.near.max(0.3);
    let far_d = (cam.far * 0.15).clamp(1.0, 3.0);

    let near_h = near_d * (cam.fov_y * 0.5).tan();
    let near_w = near_h * aspect;
    let far_h = far_d * (cam.fov_y * 0.5).tan();
    let far_w = far_h * aspect;

    let nc = pos + fwd * near_d;
    let fc = pos + fwd * far_d;

    let n = [
        nc + right * near_w + up * near_h,
        nc - right * near_w + up * near_h,
        nc - right * near_w - up * near_h,
        nc + right * near_w - up * near_h,
    ];
    let f = [
        fc + right * far_w + up * far_h,
        fc - right * far_w + up * far_h,
        fc - right * far_w - up * far_h,
        fc + right * far_w - up * far_h,
    ];

    // Near rect
    for i in 0..4 {
        let j = (i + 1) % 4;
        line_batch.push_line(n[i].into(), n[j].into(), color);
    }
    // Far rect
    for i in 0..4 {
        let j = (i + 1) % 4;
        line_batch.push_line(f[i].into(), f[j].into(), color);
    }
    // Connecting + rays from eye
    for i in 0..4 {
        line_batch.push_line(n[i].into(), f[i].into(), color);
        line_batch.push_line(pos.into(), n[i].into(), color);
    }
}

// ---- Grid drawing ----

pub fn draw_grid(line_batch: &mut LineBatch, show_grid: bool) {
    if !show_grid {
        return;
    }
    for i in -GRID_HALF_SIZE..=GRID_HALF_SIZE {
        let f = i as f32;
        let half = GRID_HALF_SIZE as f32;
        line_batch.push_line([f, 0.0, -half], [f, 0.0, half], GRID_COLOR);
        line_batch.push_line([-half, 0.0, f], [half, 0.0, f], GRID_COLOR);
    }
}
