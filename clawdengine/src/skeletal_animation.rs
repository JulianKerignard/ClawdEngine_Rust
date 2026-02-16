use glam::{Mat4, Quat, Vec3};

use crate::core::{
    AnimationChannel, AnimationClip, AnimationClipStore, AnimationProperty, EntityId,
    InterpolationMode, Skeleton, SkeletonStore, Transform, World, MAX_JOINTS,
};

// ---- Keyframe search ----

/// Binary search for the two keyframe indices surrounding `time`.
/// Returns (index_before, index_after, interpolation_factor).
fn find_keyframes(timestamps: &[f32], time: f32) -> (usize, usize, f32) {
    if timestamps.len() <= 1 {
        return (0, 0, 0.0);
    }
    let last = timestamps.len() - 1;
    if time <= timestamps[0] {
        return (0, 0, 0.0);
    }
    if time >= timestamps[last] {
        return (last, last, 0.0);
    }
    let mut lo = 0usize;
    let mut hi = last;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if timestamps[mid] <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let seg = timestamps[hi] - timestamps[lo];
    let t = if seg > 1e-8 {
        (time - timestamps[lo]) / seg
    } else {
        0.0
    };
    (lo, hi, t)
}

// ---- Value readers ----

fn component_count(prop: AnimationProperty) -> usize {
    match prop {
        AnimationProperty::Translation | AnimationProperty::Scale => 3,
        AnimationProperty::Rotation => 4,
    }
}

fn read_value(values: &[f32], idx: usize, cc: usize, is_cubic: bool) -> &[f32] {
    if is_cubic {
        let stride = 3 * cc;
        let offset = idx * stride + cc; // skip in-tangent
        &values[offset..offset + cc]
    } else {
        let offset = idx * cc;
        &values[offset..offset + cc]
    }
}

fn read_out_tangent(values: &[f32], idx: usize, cc: usize) -> &[f32] {
    let stride = 3 * cc;
    let offset = idx * stride + 2 * cc;
    &values[offset..offset + cc]
}

fn read_in_tangent(values: &[f32], idx: usize, cc: usize) -> &[f32] {
    let stride = 3 * cc;
    let offset = idx * stride;
    &values[offset..offset + cc]
}

// ---- Interpolation ----

fn interpolate_linear(prop: AnimationProperty, v0: &[f32], v1: &[f32], t: f32) -> [f32; 4] {
    match prop {
        AnimationProperty::Translation | AnimationProperty::Scale => {
            let a = Vec3::new(v0[0], v0[1], v0[2]);
            let b = Vec3::new(v1[0], v1[1], v1[2]);
            let r = a.lerp(b, t);
            [r.x, r.y, r.z, 0.0]
        }
        AnimationProperty::Rotation => {
            let a = Quat::from_xyzw(v0[0], v0[1], v0[2], v0[3]).normalize();
            let b = Quat::from_xyzw(v1[0], v1[1], v1[2], v1[3]).normalize();
            let r = a.slerp(b, t);
            [r.x, r.y, r.z, r.w]
        }
    }
}

fn interpolate_cubic(
    prop: AnimationProperty,
    v0: &[f32],
    v1: &[f32],
    out_tan0: &[f32],
    in_tan1: &[f32],
    t: f32,
    dt: f32,
) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    let cc = component_count(prop);
    let mut result = [0.0f32; 4];
    for i in 0..cc {
        result[i] = h00 * v0[i] + h10 * dt * out_tan0[i] + h01 * v1[i] + h11 * dt * in_tan1[i];
    }
    if prop == AnimationProperty::Rotation {
        let q = Quat::from_xyzw(result[0], result[1], result[2], result[3]).normalize();
        result = [q.x, q.y, q.z, q.w];
    }
    result
}

// ---- Apply to Transform ----

fn apply_value(prop: AnimationProperty, val: &[f32], bone: usize, poses: &mut [Transform]) {
    if bone >= poses.len() {
        return;
    }
    match prop {
        AnimationProperty::Translation => {
            poses[bone].position = Vec3::new(val[0], val[1], val[2]);
        }
        AnimationProperty::Rotation => {
            poses[bone].rotation = Quat::from_xyzw(val[0], val[1], val[2], val[3]).normalize();
        }
        AnimationProperty::Scale => {
            poses[bone].scale = Vec3::new(val[0], val[1], val[2]);
        }
    }
}

// ---- Channel evaluation ----

fn evaluate_channel(channel: &AnimationChannel, time: f32, local_poses: &mut [Transform]) {
    if channel.timestamps.is_empty() {
        return;
    }

    let (i0, i1, t) = find_keyframes(&channel.timestamps, time);
    let cc = component_count(channel.property);
    let is_cubic = channel.interpolation == InterpolationMode::CubicSpline;

    match channel.interpolation {
        InterpolationMode::Step => {
            let val = read_value(&channel.values, i0, cc, false);
            apply_value(channel.property, val, channel.target_bone, local_poses);
        }
        InterpolationMode::Linear => {
            let v0 = read_value(&channel.values, i0, cc, false);
            let v1 = read_value(&channel.values, i1, cc, false);
            let interpolated = interpolate_linear(channel.property, v0, v1, t);
            apply_value(
                channel.property,
                &interpolated,
                channel.target_bone,
                local_poses,
            );
        }
        InterpolationMode::CubicSpline => {
            let v0 = read_value(&channel.values, i0, cc, is_cubic);
            let v1 = read_value(&channel.values, i1, cc, is_cubic);
            let out_tan0 = read_out_tangent(&channel.values, i0, cc);
            let in_tan1 = read_in_tangent(&channel.values, i1, cc);
            let dt = if i1 > i0 {
                channel.timestamps[i1] - channel.timestamps[i0]
            } else {
                1.0
            };
            let interpolated =
                interpolate_cubic(channel.property, v0, v1, out_tan0, in_tan1, t, dt);
            apply_value(
                channel.property,
                &interpolated,
                channel.target_bone,
                local_poses,
            );
        }
    }
}

// ---- Clip evaluation ----

pub fn evaluate_clip(clip: &AnimationClip, skeleton: &Skeleton, time: f32) -> Vec<Transform> {
    let mut local_poses: Vec<Transform> = skeleton
        .bones
        .iter()
        .map(|b| b.local_bind_transform)
        .collect();

    for channel in &clip.channels {
        if channel.target_bone < local_poses.len() {
            evaluate_channel(channel, time, &mut local_poses);
        }
    }

    local_poses
}

// ---- Joint matrix computation ----

pub fn compute_joint_matrices(skeleton: &Skeleton, local_poses: &[Transform]) -> Vec<Mat4> {
    let bone_count = skeleton.bones.len();
    let mut global_transforms = vec![Mat4::IDENTITY; bone_count];

    for i in 0..bone_count {
        let local = local_poses[i];
        let local_mat =
            Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.position);

        global_transforms[i] = match skeleton.bones[i].parent {
            Some(parent_idx) => global_transforms[parent_idx] * local_mat,
            None => local_mat,
        };
    }

    (0..bone_count)
        .map(|i| global_transforms[i] * skeleton.bones[i].inverse_bind_matrix)
        .collect()
}

pub fn joint_matrices_to_uniform(matrices: &[Mat4]) -> [[[f32; 4]; 4]; MAX_JOINTS] {
    let identity = Mat4::IDENTITY.to_cols_array_2d();
    let mut result = [identity; MAX_JOINTS];
    for (i, mat) in matrices.iter().enumerate().take(MAX_JOINTS) {
        result[i] = mat.to_cols_array_2d();
    }
    result
}

// ---- Per-frame system ----

pub fn skeletal_animation_system(
    world: &mut World,
    dt: f32,
    skeleton_store: &SkeletonStore,
    clip_store: &AnimationClipStore,
) -> Vec<(EntityId, [[[f32; 4]; 4]; MAX_JOINTS])> {
    let mut results = Vec::new();

    let entities: Vec<EntityId> = world
        .iter_entities()
        .filter(|&eid| world.get_skeletal_animator(eid).is_some())
        .collect();

    for eid in entities {
        let (skeleton_id, clip_id, playing, looping, speed, current_time, duration) = {
            let anim = match world.get_skeletal_animator(eid) {
                Some(a) => a,
                None => continue,
            };

            let skel_id = match anim.skeleton_id {
                Some(id) => id,
                None => continue,
            };

            let clip_idx = match anim.active_clip {
                Some(idx) if idx < anim.clip_ids.len() => anim.clip_ids[idx],
                _ => {
                    // No active clip — evaluate bind pose
                    if let Some(skeleton) = skeleton_store.get(skel_id) {
                        let local_poses: Vec<Transform> = skeleton
                            .bones
                            .iter()
                            .map(|b| b.local_bind_transform)
                            .collect();
                        let joint_mats = compute_joint_matrices(skeleton, &local_poses);
                        results.push((eid, joint_matrices_to_uniform(&joint_mats)));
                    }
                    continue;
                }
            };

            let duration = match clip_store.get(clip_idx) {
                Some(c) => c.duration,
                None => continue,
            };

            (
                skel_id,
                clip_idx,
                anim.playing,
                anim.loop_animation,
                anim.speed,
                anim.current_time,
                duration,
            )
        };

        let skeleton = match skeleton_store.get(skeleton_id) {
            Some(s) => s,
            None => continue,
        };
        let clip = match clip_store.get(clip_id) {
            Some(c) => c,
            None => continue,
        };

        // Advance time only if playing
        let mut new_time = current_time;
        if playing && duration > 0.0 {
            new_time += dt * speed;
            if new_time > duration {
                if looping {
                    new_time %= duration;
                } else {
                    new_time = duration;
                }
            }
            if new_time < 0.0 {
                new_time = if looping {
                    duration + (new_time % duration)
                } else {
                    0.0
                };
            }
        }

        let local_poses = evaluate_clip(clip, skeleton, new_time);
        let joint_mats = compute_joint_matrices(skeleton, &local_poses);
        results.push((eid, joint_matrices_to_uniform(&joint_mats)));

        if let Some(anim) = world.get_skeletal_animator_mut(eid) {
            anim.current_time = new_time;
            if playing && new_time >= duration && !looping {
                anim.playing = false;
            }
        }
    }

    results
}
