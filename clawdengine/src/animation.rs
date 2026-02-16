use crate::core::{EntityId, Keyframe, World};

pub fn animation_system(world: &mut World, dt: f32) {
    let ids: Vec<EntityId> = world
        .iter_entities()
        .filter(|&e| world.get_animator(e).map_or(false, |a| a.playing))
        .collect();

    for eid in ids {
        let (kfs, looping, speed, time) = {
            let a = world.get_animator(eid).unwrap();
            if a.keyframes.len() < 2 {
                continue;
            }
            (
                a.keyframes.clone(),
                a.loop_animation,
                a.speed,
                a.current_time,
            )
        };

        let max_t = match kfs.last() {
            Some(kf) if kf.time > 0.0 => kf.time,
            _ => continue,
        };

        let mut new_time = time + dt * speed;
        let mut still_playing = true;
        if new_time > max_t {
            if looping && max_t > 0.0 {
                new_time = new_time.rem_euclid(max_t);
            } else {
                new_time = max_t;
                still_playing = false;
            }
        } else if new_time < 0.0 && looping && max_t > 0.0 {
            new_time = new_time.rem_euclid(max_t);
        }

        let (prev, next) = find_surrounding(&kfs, new_time);
        let seg_len = next.time - prev.time;
        let t = if seg_len > 1e-6 {
            (new_time - prev.time) / seg_len
        } else {
            0.0
        };

        if let Some(transform) = world.get_transform_mut(eid) {
            if let (Some(p0), Some(p1)) = (prev.position, next.position) {
                transform.position = p0.lerp(p1, t);
            }
            if let (Some(r0), Some(r1)) = (prev.rotation, next.rotation) {
                transform.rotation = r0.slerp(r1, t);
            }
            if let (Some(s0), Some(s1)) = (prev.scale, next.scale) {
                transform.scale = s0.lerp(s1, t);
            }
        }

        if let Some(a) = world.get_animator_mut(eid) {
            a.current_time = new_time;
            if !still_playing {
                a.playing = false;
            }
        }
    }
}

fn find_surrounding<'a>(kfs: &'a [Keyframe], time: f32) -> (&'a Keyframe, &'a Keyframe) {
    for i in 0..kfs.len() - 1 {
        if kfs[i].time <= time && kfs[i + 1].time >= time {
            return (&kfs[i], &kfs[i + 1]);
        }
    }
    let last = kfs.len() - 1;
    (&kfs[last - 1], &kfs[last])
}
