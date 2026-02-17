use egui::{Color32, DragValue, Slider};

use crate::editor::layout::{EditorTabViewer, component_section, property_row};
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_audio_source(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_audio_source(eid).is_none() {
            return;
        }
        const AUDIO_ACCENT: Color32 = Color32::from_rgb(0xF9, 0xE2, 0xAF);
        let remove_audio = component_section(ui, "audio", "A", "AudioSource", AUDIO_ACCENT, true, |ui| {
            let Some(audio) = self.world.get_audio_source_mut(eid) else { return; };

            property_row(ui, "File", |ui| {
                let mut remove = false;
                if let Some(ref path) = audio.audio_path {
                    let display = path.rsplit('/').next().unwrap_or(path);
                    ui.label(egui::RichText::new(display).color(theme::TEXT_PRIMARY).size(11.0));
                    if ui.small_button("x").on_hover_text("Remove").clicked() {
                        remove = true;
                    }
                } else {
                    ui.label(egui::RichText::new("None").color(theme::TEXT_DISABLED).size(11.0));
                }
                if remove { audio.audio_path = None; }
            });

            let browse = ui.small_button("Browse Audio...");
            egui::Popup::from_toggle_button_response(&browse)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui: &mut egui::Ui| {
                    ui.set_min_width(200.0);
                    fn scan(dir: &std::path::Path, files: &mut Vec<String>) {
                        if let Ok(entries) = std::fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.is_dir() { scan(&p, files); }
                                else if let Some(ext) = p.extension() {
                                    let ext = ext.to_string_lossy().to_lowercase();
                                    if ext == "wav" || ext == "ogg" {
                                        files.push(p.to_string_lossy().into_owned());
                                    }
                                }
                            }
                        }
                    }
                    let mut files = Vec::new();
                    scan(std::path::Path::new("assets"), &mut files);
                    files.sort();
                    if files.is_empty() {
                        ui.label(egui::RichText::new("No .wav/.ogg files").color(theme::TEXT_DISABLED));
                    }
                    for path in &files {
                        let display = path.rsplit('/').next().unwrap_or(path);
                        if ui.button(display).clicked() {
                            audio.audio_path = Some(path.clone());
                        }
                    }
                });

            property_row(ui, "Volume", |ui| {
                ui.add(Slider::new(&mut audio.volume, 0.0..=1.0));
            });
            property_row(ui, "Pitch", |ui| {
                ui.add(DragValue::new(&mut audio.pitch).speed(0.01).range(0.5..=2.0));
            });
            property_row(ui, "Loop", |ui| {
                ui.checkbox(&mut audio.loop_audio, "");
            });
            property_row(ui, "Play on Start", |ui| {
                ui.checkbox(&mut audio.play_on_start, "");
            });
            property_row(ui, "Spatial", |ui| {
                ui.checkbox(&mut audio.spatial, "");
            });
            if audio.spatial {
                property_row(ui, "Max Distance", |ui| {
                    ui.add(DragValue::new(&mut audio.max_distance).speed(0.5).range(0.1..=100.0));
                });
            }
        });
        if remove_audio {
            self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
            self.world.remove_audio_source(eid);
        }
    }
}
