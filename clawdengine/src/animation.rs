use crate::core::{EntityId, Keyframe, World};

const MIN_KEYFRAME_SEGMENT: f32 = 1e-6;

pub fn animation_system(world: &mut World, dt: f32, entity_buf: &mut Vec<EntityId>) {
    entity_buf.clear();
    entity_buf.extend(
        world.iter_entities()
            .filter(|&e| world.get_animator(e).map_or(false, |a| a.playing)),
    );

    for i in 0..entity_buf.len() {
        let eid = entity_buf[i];
        let (looping, speed, time, max_t) = {
            let a = world.get_animator(eid).unwrap();
            if a.keyframes.len() < 2 {
                continue;
            }
            let max_t = match a.keyframes.last() {
                Some(kf) if kf.time > 0.0 => kf.time,
                _ => continue,
            };
            (a.loop_animation, a.speed, a.current_time, max_t)
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

        // Borrow keyframes briefly to copy just the 2 surrounding frames (not the whole Vec)
        let (prev, next) = {
            let a = world.get_animator(eid).unwrap();
            let (p, n) = find_surrounding(&a.keyframes, new_time);
            (p.clone(), n.clone())
        };
        let seg_len = next.time - prev.time;
        let t = if seg_len > MIN_KEYFRAME_SEGMENT {
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
    let last = kfs.len() - 1;
    if time <= kfs[0].time {
        return (&kfs[0], &kfs[1]);
    }
    if time >= kfs[last].time {
        return (&kfs[last - 1], &kfs[last]);
    }
    let mut lo = 0usize;
    let mut hi = last;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if kfs[mid].time <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (&kfs[lo], &kfs[hi])
}
