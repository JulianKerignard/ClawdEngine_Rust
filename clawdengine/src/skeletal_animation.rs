use glam::{Mat4, Quat, Vec3};

const MIN_KEYFRAME_SEGMENT: f32 = 1e-6;

use crate::core::{
    AnimationChannel, AnimationClip, AnimationClipStore, AnimationProperty,
    AnimatorControllerStore, EntityId, InterpolationMode, Skeleton, SkeletonStore,
    StateChange, Transform, World, MAX_JOINTS,
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
    let t = if seg > MIN_KEYFRAME_SEGMENT {
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

/// Evaluate clip into a reusable buffer (zero-alloc per frame).
pub fn evaluate_clip_into(
    clip: &AnimationClip,
    skeleton: &Skeleton,
    time: f32,
    local_poses: &mut Vec<Transform>,
) {
    local_poses.clear();
    local_poses.extend(skeleton.bones.iter().map(|b| b.local_bind_transform));
    for channel in &clip.channels {
        if channel.target_bone < local_poses.len() {
            evaluate_channel(channel, time, local_poses);
        }
    }
}

// ---- Joint matrix computation ----

/// Compute joint matrices into reusable buffers (zero-alloc per frame).
pub fn compute_joint_matrices_into(
    skeleton: &Skeleton,
    local_poses: &[Transform],
    global_buf: &mut Vec<Mat4>,
    joint_buf: &mut Vec<Mat4>,
) {
    let bone_count = skeleton.bones.len();
    global_buf.clear();
    global_buf.resize(bone_count, Mat4::IDENTITY);

    for i in 0..bone_count {
        let local = local_poses[i];
        let local_mat =
            Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.position);

        global_buf[i] = match skeleton.bones[i].parent {
            Some(parent_idx) => global_buf[parent_idx] * local_mat,
            None => local_mat,
        };
    }

    joint_buf.clear();
    joint_buf.extend(
        (0..bone_count).map(|i| global_buf[i] * skeleton.bones[i].inverse_bind_matrix),
    );
}

pub fn joint_matrices_to_uniform(matrices: &[Mat4]) -> [[[f32; 4]; 4]; MAX_JOINTS] {
    let identity = Mat4::IDENTITY.to_cols_array_2d();
    let mut result = [identity; MAX_JOINTS];
    for (i, mat) in matrices.iter().enumerate().take(MAX_JOINTS) {
        result[i] = mat.to_cols_array_2d();
    }
    result
}

// ---- Pose blending ----

/// Blend two sets of local poses. t=0.0 = fully `a`, t=1.0 = fully `b`.
fn blend_poses(a: &[Transform], b: &[Transform], t: f32, out: &mut Vec<Transform>) {
    out.clear();
    let len = a.len().min(b.len());
    for i in 0..len {
        out.push(Transform {
            position: a[i].position.lerp(b[i].position, t),
            rotation: a[i].rotation.slerp(b[i].rotation, t),
            scale: a[i].scale.lerp(b[i].scale, t),
        });
    }
}

// ---- Per-frame system ----

/// One-time flag to log skeletal system summary once per session.
static FIRST_EVAL_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[allow(clippy::too_many_arguments)]
pub fn skeletal_animation_system(
    world: &mut World,
    dt: f32,
    skeleton_store: &SkeletonStore,
    clip_store: &AnimationClipStore,
    controller_store: &AnimatorControllerStore,
    entity_buf: &mut Vec<EntityId>,
    results_buf: &mut Vec<(EntityId, [[[f32; 4]; 4]; MAX_JOINTS])>,
    local_poses_buf: &mut Vec<Transform>,
    global_buf: &mut Vec<Mat4>,
    joint_buf: &mut Vec<Mat4>,
) {
    results_buf.clear();
    let mut prev_poses_buf: Vec<Transform> = Vec::new();
    let mut blended_poses_buf: Vec<Transform> = Vec::new();

    entity_buf.clear();
    entity_buf.extend(
        world
            .iter_entities()
            .filter(|&eid| world.get_skeletal_animator(eid).is_some()),
    );

    // Log once when skeletal entities first appear
    if !entity_buf.is_empty() && !FIRST_EVAL_LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        log::info!("[SkelAnim] System running: {} entities with SkeletalAnimator, dt={:.4}s", entity_buf.len(), dt);
        for &eid in entity_buf.iter() {
            if let Some(anim) = world.get_skeletal_animator(eid) {
                log::info!("[SkelAnim]   Entity {:?}: skeleton={:?}, clips={}, active={:?}, playing={}",
                    eid, anim.skeleton_name, anim.clip_ids.len(), anim.active_clip, anim.playing);
            }
        }
    }

    // ---- Parent → children sync ----
    // When a root entity has SkeletalAnimator (for Inspector control),
    // propagate its playback state to children so the user can control
    // the animation from the parent entity in the Inspector.
    {
        let mut syncs: Vec<(
            EntityId,
            bool,   // playing
            bool,   // loop
            f32,    // speed
            Option<usize>,   // active_clip
            Option<String>,  // active_clip_name
            f32,    // current_time
        )> = Vec::new();

        for &eid in entity_buf.iter() {
            if world.get_children(eid).is_empty() {
                continue;
            }
            if let Some(pa) = world.get_skeletal_animator(eid) {
                syncs.push((
                    eid,
                    pa.playing,
                    pa.loop_animation,
                    pa.speed,
                    pa.active_clip,
                    pa.active_clip_name.clone(),
                    pa.current_time,
                ));
            }
        }

        for (parent_eid, playing, looping, speed, active_clip, active_clip_name, current_time) in syncs {
            let children: Vec<EntityId> = world.get_children(parent_eid).to_vec();
            for child in children {
                if let Some(ca) = world.get_skeletal_animator_mut(child) {
                    ca.playing = playing;
                    ca.loop_animation = looping;
                    ca.speed = speed;
                    ca.active_clip = active_clip;
                    ca.active_clip_name = active_clip_name.clone();
                    ca.current_time = current_time;
                }
            }
        }
    }

    for &eid in entity_buf.iter() {
        // --- AnimatorController evaluation ---
        // If entity has a controller, evaluate it to determine state transitions
        {
            let controller_info = {
                let anim = match world.get_skeletal_animator(eid) {
                    Some(a) => a,
                    None => continue,
                };
                anim.controller_id.and_then(|cid| {
                    controller_store.get(cid).map(|ctrl| (cid, ctrl.clone()))
                })
            };

            if let Some((_cid, controller)) = controller_info {
                let anim = match world.get_skeletal_animator_mut(eid) {
                    Some(a) => a,
                    None => continue,
                };

                if let Some(ref mut ctrl_state) = anim.controller_state {
                    // Get current clip duration for exit_time evaluation
                    let clip_duration = controller
                        .states
                        .get(ctrl_state.current_state)
                        .and_then(|s| s.clip_name.as_ref())
                        .and_then(|name| clip_store.find_by_name(name))
                        .and_then(|id| clip_store.get(id))
                        .map(|c| c.duration)
                        .unwrap_or(1.0);

                    let change = ctrl_state.evaluate(&controller, dt, clip_duration);

                    match change {
                        StateChange::TransitionStarted {
                            from: _,
                            to,
                            blend_duration: _,
                        } => {
                            // Update the active clip to the new state's clip
                            if let Some(new_state) = controller.states.get(to) {
                                if let Some(ref clip_name) = new_state.clip_name {
                                    if let Some(idx) =
                                        anim.clip_names.iter().position(|n| n == clip_name)
                                    {
                                        anim.active_clip = Some(idx);
                                        anim.active_clip_name = Some(clip_name.clone());
                                    }
                                }
                                anim.speed = new_state.speed;
                                anim.loop_animation = new_state.loop_animation;
                            }
                        }
                        StateChange::BlendCompleted { .. } => {
                            // Blend finished, nothing special needed
                        }
                        StateChange::None => {}
                    }
                }
            }
        }

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
                    // No active clip — evaluate bind pose using buffers
                    log::trace!("[SkelAnim] Entity {:?}: no active clip, evaluating bind pose", eid);
                    if let Some(skeleton) = skeleton_store.get(skel_id) {
                        local_poses_buf.clear();
                        local_poses_buf
                            .extend(skeleton.bones.iter().map(|b| b.local_bind_transform));
                        compute_joint_matrices_into(
                            skeleton,
                            local_poses_buf,
                            global_buf,
                            joint_buf,
                        );
                        // Only push for leaf entities (not parent/group nodes)
                        if world.get_children(eid).is_empty() {
                            results_buf.push((eid, joint_matrices_to_uniform(joint_buf)));
                        }
                        if let Some(anim) = world.get_skeletal_animator_mut(eid) {
                            anim.current_local_poses = local_poses_buf.clone();
                        }
                    } else {
                        log::warn!("[SkelAnim] Entity {:?}: skeleton_id={} not found in store (store has {} skeletons)",
                            eid, skel_id, skeleton_store.len());
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
            None => {
                log::warn!("[SkelAnim] Entity {:?}: skeleton store id {} not found", eid, skeleton_id);
                continue;
            }
        };
        let clip = match clip_store.get(clip_id) {
            Some(c) => c,
            None => {
                log::warn!("[SkelAnim] Entity {:?}: clip store id {} not found", eid, clip_id);
                continue;
            }
        };

        // Advance time only if playing
        let mut new_time = current_time;
        if playing && duration > 0.0 {
            new_time += dt * speed;
            if new_time > duration {
                if looping && duration > 0.0 {
                    new_time = new_time.rem_euclid(duration);
                } else {
                    new_time = duration;
                }
            }
            if new_time < 0.0 {
                new_time = if looping && duration > 0.0 {
                    new_time.rem_euclid(duration)
                } else {
                    0.0
                };
            }
        }

        evaluate_clip_into(clip, skeleton, new_time, local_poses_buf);

        // Check if we're blending between two clips (controller transition)
        let (needs_blend, prev_clip_name, blend_t, prev_time) = {
            let anim = world.get_skeletal_animator(eid);
            let info = anim
                .and_then(|a| {
                    let cs = a.controller_state.as_ref()?;
                    if !cs.is_blending || cs.previous_state.is_none() {
                        return None;
                    }
                    let ctrl_id = a.controller_id?;
                    let controller = controller_store.get(ctrl_id)?;
                    let prev_name = cs
                        .previous_state
                        .and_then(|ps| controller.states.get(ps))
                        .and_then(|s| s.clip_name.clone());
                    Some((prev_name, cs.blend_progress, cs.previous_state_time))
                });
            match info {
                Some((name, t, pt)) => (true, name, t, pt),
                None => (false, None, 0.0, 0.0),
            }
        };

        let final_poses: &[Transform] = if needs_blend {
            if let Some(prev_name) = prev_clip_name {
                let blended = clip_store
                    .find_by_name(&prev_name)
                    .and_then(|id| clip_store.get(id))
                    .map(|prev_clip| {
                        evaluate_clip_into(prev_clip, skeleton, prev_time, &mut prev_poses_buf);
                        blend_poses(&prev_poses_buf, local_poses_buf, blend_t, &mut blended_poses_buf);
                    });
                if blended.is_some() {
                    &blended_poses_buf
                } else {
                    local_poses_buf
                }
            } else {
                local_poses_buf
            }
        } else {
            local_poses_buf
        };

        compute_joint_matrices_into(skeleton, final_poses, global_buf, joint_buf);

        // Only push rendering results for leaf entities (actual skinned meshes).
        // Parent/group entities have SkeletalAnimator for Inspector control only.
        if world.get_children(eid).is_empty() {
            results_buf.push((eid, joint_matrices_to_uniform(joint_buf)));
        }

        if let Some(anim) = world.get_skeletal_animator_mut(eid) {
            anim.current_time = new_time;
            anim.current_local_poses = final_poses.to_vec();
            if playing && new_time >= duration && !looping {
                anim.playing = false;
            }
        }
    }
}
