use crate::core::{EntityId, World};
use crate::renderer::SceneRenderer;

use super::context::{EditorContext, EditorTool, GizmoAxis, GizmoDragState};
use super::picking;

/// Walk up parent chain: if parent has no mesh (group entity), select it instead.
fn resolve_root_parent(eid: EntityId, world: &World) -> EntityId {
    let mut current = eid;
    while let Some(parent) = world.get_parent(current) {
        if world.get_mesh_renderer(parent).is_none() {
            current = parent;
        } else {
            break;
        }
    }
    current
}

/// Collect initial positions for all selected entities.
fn collect_positions(editor_ctx: &EditorContext, world: &World) -> Vec<(crate::core::EntityId, glam::Vec3)> {
    editor_ctx.selected_entities.iter().filter_map(|&eid| {
        world.get_transform(eid).map(|t| (eid, t.position))
    }).collect()
}

/// Collect initial (pos, rotation) for all selected entities.
fn collect_transforms(editor_ctx: &EditorContext, world: &World) -> Vec<(crate::core::EntityId, glam::Vec3, glam::Quat)> {
    editor_ctx.selected_entities.iter().filter_map(|&eid| {
        world.get_transform(eid).map(|t| (eid, t.position, t.rotation))
    }).collect()
}

/// Collect initial (pos, scale) for all selected entities.
fn collect_scales(editor_ctx: &EditorContext, world: &World) -> Vec<(crate::core::EntityId, glam::Vec3, glam::Vec3)> {
    editor_ctx.selected_entities.iter().filter_map(|&eid| {
        world.get_transform(eid).map(|t| (eid, t.position, t.scale))
    }).collect()
}

/// Handle left-click: attempt gizmo pick first, then fall back to entity pick.
pub fn handle_gizmo_press(
    editor_ctx: &mut EditorContext,
    world: &World,
    scene: &SceneRenderer,
    mx: f32,
    my: f32,
    vp_width: f32,
    vp_height: f32,
    shift_held: bool,
) {
    let mut gizmo_hit = false;

    if let Some(center) = editor_ctx.selection_center(world) {
        match editor_ctx.active_tool {
            EditorTool::Move | EditorTool::Scale => {
                if let Some((axis, t)) = picking::pick_gizmo_axis(
                    &scene.camera, vp_width, vp_height, mx, my, center,
                ) {
                    editor_ctx.gizmo_drag = Some(
                        if editor_ctx.active_tool == EditorTool::Move {
                            GizmoDragState::Move {
                                axis,
                                initial_t: t,
                                center,
                                initial_positions: collect_positions(editor_ctx, world),
                            }
                        } else {
                            GizmoDragState::Scale {
                                axis,
                                initial_t: t,
                                center,
                                initial_scales: collect_scales(editor_ctx, world),
                            }
                        },
                    );
                    gizmo_hit = true;
                }
            }
            EditorTool::Rotate => {
                if let Some((axis, angle)) = picking::pick_gizmo_ring(
                    &scene.camera, vp_width, vp_height, mx, my, center,
                ) {
                    editor_ctx.gizmo_drag = Some(GizmoDragState::Rotate {
                        axis,
                        initial_angle: angle,
                        center,
                        initial_transforms: collect_transforms(editor_ctx, world),
                    });
                    gizmo_hit = true;
                }
            }
            EditorTool::Select => {}
        }
    }

    // Push undo snapshot before gizmo drag modifies transforms
    if gizmo_hit {
        editor_ctx.undo_stack.push(world.snapshot(), editor_ctx.selected_entities.clone());
    }

    if !gizmo_hit {
        let picked = picking::pick_entity(
            &scene.camera, vp_width, vp_height, mx, my, world, &scene.mesh_store,
        );
        // If picked entity has a group parent (no mesh), select the root instead
        let resolved = picked.map(|eid| resolve_root_parent(eid, world));
        match resolved {
            Some(eid) if shift_held => editor_ctx.toggle_select(eid),
            Some(eid) => editor_ctx.select(eid),
            None => editor_ctx.deselect_all(),
        }
    }
}

/// Handle left-held drag: update transforms for ALL selected entities.
pub fn handle_gizmo_drag(
    editor_ctx: &EditorContext,
    world: &mut World,
    scene: &SceneRenderer,
    mx: f32,
    my: f32,
    vp_width: f32,
    vp_height: f32,
) {
    let Some(ref drag) = editor_ctx.gizmo_drag else {
        return;
    };
    match drag {
        GizmoDragState::Move {
            axis, initial_t, center, initial_positions,
        } => {
            let current_t = picking::project_ray_onto_axis(
                &scene.camera, vp_width, vp_height, mx, my, *center, *axis,
            );
            let delta = axis.direction() * (current_t - initial_t);
            for &(eid, init_pos) in initial_positions {
                if let Some(t) = world.get_transform_mut(eid) {
                    t.position = init_pos + delta;
                }
            }
        }
        GizmoDragState::Rotate {
            axis, initial_angle, center, initial_transforms,
        } => {
            let current_angle = picking::project_ray_onto_ring(
                &scene.camera, vp_width, vp_height, mx, my, *center, *axis,
            );
            let delta_angle = current_angle - initial_angle;
            let rot_delta = glam::Quat::from_axis_angle(axis.direction(), delta_angle);
            for &(eid, init_pos, init_rot) in initial_transforms {
                if let Some(t) = world.get_transform_mut(eid) {
                    // Rotate individual rotation
                    t.rotation = rot_delta * init_rot;
                    // Orbit position around center
                    let offset = init_pos - *center;
                    t.position = *center + rot_delta * offset;
                }
            }
        }
        GizmoDragState::Scale {
            axis, initial_t, center, initial_scales,
        } => {
            let current_t = picking::project_ray_onto_axis(
                &scene.camera, vp_width, vp_height, mx, my, *center, *axis,
            );
            let factor = (1.0 + (current_t - initial_t)).max(0.01);
            for &(eid, init_pos, init_scale) in initial_scales {
                if let Some(t) = world.get_transform_mut(eid) {
                    let mut new_scale = init_scale;
                    match axis {
                        GizmoAxis::X => new_scale.x = (init_scale.x * factor).max(0.01),
                        GizmoAxis::Y => new_scale.y = (init_scale.y * factor).max(0.01),
                        GizmoAxis::Z => new_scale.z = (init_scale.z * factor).max(0.01),
                    }
                    t.scale = new_scale;
                    // Adjust position relative to center
                    let offset = init_pos - *center;
                    let mut scaled_offset = offset;
                    match axis {
                        GizmoAxis::X => scaled_offset.x *= factor,
                        GizmoAxis::Y => scaled_offset.y *= factor,
                        GizmoAxis::Z => scaled_offset.z *= factor,
                    }
                    t.position = *center + scaled_offset;
                }
            }
        }
    }
}

/// Clear gizmo drag state when the mouse button is released.
pub fn handle_gizmo_release(editor_ctx: &mut EditorContext) {
    editor_ctx.gizmo_drag = None;
}

/// Update the hovered gizmo axis for visual highlighting (called each frame).
pub fn update_hovered_axis(
    editor_ctx: &mut EditorContext,
    world: &World,
    scene: &SceneRenderer,
    mx: f32,
    my: f32,
    vp_width: f32,
    vp_height: f32,
) {
    if editor_ctx.gizmo_drag.is_some() {
        return;
    }
    let Some(origin) = editor_ctx.selection_center(world) else {
        editor_ctx.hovered_gizmo_axis = None;
        return;
    };
    editor_ctx.hovered_gizmo_axis = match editor_ctx.active_tool {
        EditorTool::Move | EditorTool::Scale => {
            picking::pick_gizmo_axis(&scene.camera, vp_width, vp_height, mx, my, origin)
                .map(|(axis, _)| axis)
        }
        EditorTool::Rotate => {
            picking::pick_gizmo_ring(&scene.camera, vp_width, vp_height, mx, my, origin)
                .map(|(axis, _)| axis)
        }
        _ => None,
    };
}
