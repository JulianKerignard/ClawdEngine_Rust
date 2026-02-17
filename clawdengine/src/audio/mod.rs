use std::collections::HashMap;
use std::io::BufReader;

use glam::Vec3;
use rodio::Source;

use crate::core::{EntityId, World};

pub struct AudioSystem {
    stream: rodio::OutputStream,
    handles: HashMap<u32, rodio::Sink>,
    paths: HashMap<u32, String>,
}

impl AudioSystem {
    pub fn new() -> Option<Self> {
        let stream = rodio::OutputStreamBuilder::open_default_stream().ok()?;
        Some(Self {
            stream,
            handles: HashMap::new(),
            paths: HashMap::new(),
        })
    }

    pub fn update(&mut self, world: &mut World, is_playing: bool) {
        if !is_playing {
            if !self.handles.is_empty() {
                self.stop_all();
            }
            return;
        }

        // Require an active AudioListener (like Unity)
        let Some((listener_pos, listener_vol)) = find_listener(world) else {
            if !self.handles.is_empty() {
                self.stop_all();
            }
            return;
        };

        let entities: Vec<EntityId> = world.iter_entities().collect();
        let mut active: std::collections::HashSet<u32> = std::collections::HashSet::new();

        for &eid in &entities {
            let Some(audio) = world.get_audio_source(eid) else { continue };
            let idx = eid.index;
            active.insert(idx);

            if audio.is_playing {
                let path_changed = self.paths.get(&idx)
                    .is_none_or(|p| audio.audio_path.as_deref() != Some(p.as_str()));

                if !self.handles.contains_key(&idx) || path_changed {
                    // Stop old handle
                    if let Some(sink) = self.handles.remove(&idx) {
                        sink.stop();
                    }
                    self.paths.remove(&idx);

                    // Start new playback
                    if let Some(ref path) = audio.audio_path {
                        self.start_sink(idx, path, audio.volume * listener_vol, audio.pitch, audio.loop_audio);
                    }
                } else if let Some(sink) = self.handles.get(&idx) {
                    let mut vol = audio.volume * listener_vol;
                    if audio.spatial {
                        if let Some(pos) = world.get_transform(eid).map(|t| t.position) {
                            let dist = (pos - listener_pos).length();
                            vol *= distance_attenuation(dist, audio.max_distance);
                        }
                    }
                    sink.set_volume(vol);
                    sink.set_speed(audio.pitch);
                }

                // Check if playback finished (non-looping)
                if let Some(sink) = self.handles.get(&idx) {
                    if sink.empty() && !audio.loop_audio {
                        if let Some(a) = world.get_audio_source_mut(eid) {
                            a.is_playing = false;
                        }
                        self.handles.remove(&idx);
                        self.paths.remove(&idx);
                    }
                }
            } else {
                if let Some(sink) = self.handles.remove(&idx) {
                    sink.stop();
                }
                self.paths.remove(&idx);
            }
        }

        // Cleanup handles for destroyed entities
        let to_remove: Vec<u32> = self.handles.keys()
            .filter(|k| !active.contains(k))
            .copied()
            .collect();
        for idx in to_remove {
            if let Some(sink) = self.handles.remove(&idx) {
                sink.stop();
            }
            self.paths.remove(&idx);
        }
    }

    pub fn stop_all(&mut self) {
        for (_, sink) in self.handles.drain() {
            sink.stop();
        }
        self.paths.clear();
    }

    fn start_sink(&mut self, idx: u32, path: &str, volume: f32, pitch: f32, looping: bool) {
        let Ok(file) = std::fs::File::open(path) else { return };
        let reader = BufReader::new(file);
        let Ok(source) = rodio::Decoder::new(reader) else {
            log::warn!("Failed to decode audio: {}", path);
            return;
        };
        let sink = rodio::Sink::connect_new(self.stream.mixer());
        sink.set_volume(volume);
        sink.set_speed(pitch);
        if looping {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }
        self.handles.insert(idx, sink);
        self.paths.insert(idx, path.to_string());
    }
}

pub fn start_play_mode_audio(world: &mut World) {
    let entities: Vec<EntityId> = world.iter_entities().collect();
    for eid in entities {
        if let Some(audio) = world.get_audio_source_mut(eid) {
            if audio.play_on_start {
                audio.is_playing = true;
            }
        }
    }
}

fn find_listener(world: &World) -> Option<(Vec3, f32)> {
    for eid in world.iter_entities() {
        if let Some(listener) = world.get_audio_listener(eid) {
            if listener.active {
                let pos = world.get_transform(eid).map(|t| t.position).unwrap_or(Vec3::ZERO);
                return Some((pos, listener.volume));
            }
        }
    }
    None
}

fn distance_attenuation(dist: f32, max_dist: f32) -> f32 {
    if max_dist <= 0.0 || !dist.is_finite() { return 1.0; }
    let dist = dist.max(0.0);
    let ratio = (dist / max_dist).min(1.0);
    let falloff = (1.0 - ratio.powi(4)).powi(2) / (dist * dist + 1.0);
    falloff.clamp(0.0, 1.0)
}
