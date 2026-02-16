use egui::{Color32, ComboBox, DragValue};

use crate::editor::layout::{EditorTabViewer, component_section, property_row};
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_animator(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_animator(eid).is_none() {
            return;
        }
        const ANIM_ACCENT: Color32 = Color32::from_rgb(0xA6, 0xE3, 0xA1);
        let remove_anim = component_section(ui, "animator", "An", "Animator", ANIM_ACCENT, true, |ui| {
            let anim = self.world.get_animator_mut(eid).unwrap();

            property_row(ui, "Playing", |ui| {
                ui.checkbox(&mut anim.playing, "");
            });
            property_row(ui, "Loop", |ui| {
                ui.checkbox(&mut anim.loop_animation, "");
            });
            property_row(ui, "Speed", |ui| {
                ui.add(DragValue::new(&mut anim.speed).speed(0.05).range(0.1..=10.0));
            });
            if anim.current_time > 0.0 {
                property_row(ui, "Time", |ui| {
                    ui.label(format!("{:.2}s", anim.current_time));
                });
            }

            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("Keyframes ({})", anim.keyframes.len())).color(theme::TEXT_DISABLED).size(11.0));

            let mut remove_idx: Option<usize> = None;
            for (i, kf) in anim.keyframes.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{:.2}s", kf.time)).monospace().size(11.0));
                    if kf.position.is_some() {
                        ui.label(egui::RichText::new("P").color(Color32::from_rgb(0xF3, 0x8B, 0xA8)).size(10.0));
                    }
                    if kf.rotation.is_some() {
                        ui.label(egui::RichText::new("R").color(Color32::from_rgb(0xA6, 0xE3, 0xA1)).size(10.0));
                    }
                    if kf.scale.is_some() {
                        ui.label(egui::RichText::new("S").color(Color32::from_rgb(0x89, 0xB4, 0xFA)).size(10.0));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("x").size(10.0).color(theme::TEXT_DISABLED)).frame(false)).clicked() {
                            remove_idx = Some(i);
                        }
                    });
                });
            }
            if let Some(idx) = remove_idx {
                anim.keyframes.remove(idx);
            }

            let capture_time = if anim.keyframes.is_empty() {
                0.0
            } else {
                anim.keyframes.last().unwrap().time + 1.0
            };
            if ui.small_button("+ Capture Keyframe").clicked() {
                if let Some(t) = self.world.get_transform(eid) {
                    let kf = crate::core::Keyframe {
                        time: capture_time,
                        position: Some(t.position),
                        rotation: Some(t.rotation),
                        scale: Some(t.scale),
                    };
                    if let Some(a) = self.world.get_animator_mut(eid) {
                        a.keyframes.push(kf);
                    }
                }
            }
        });
        if remove_anim {
            self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
            self.world.remove_animator(eid);
        }
    }

    pub(super) fn inspector_skeletal_animator(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_skeletal_animator(eid).is_none() {
            return;
        }
        const SKEL_ACCENT: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7);
        let remove_skel = component_section(ui, "skeletal_animator", "Sk", "SkeletalAnimator", SKEL_ACCENT, true, |ui| {
            let sa = self.world.get_skeletal_animator_mut(eid).unwrap();

            property_row(ui, "Skeleton", |ui| {
                ui.label(sa.skeleton_name.as_deref().unwrap_or("(none)"));
            });

            if !sa.clip_names.is_empty() {
                property_row(ui, "Clip", |ui| {
                    let current = sa.active_clip_name.as_deref().unwrap_or("(none)");
                    ComboBox::from_id_salt("skel_clip_select")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (i, name) in sa.clip_names.iter().enumerate() {
                                if ui.selectable_label(sa.active_clip == Some(i), name).clicked() {
                                    sa.active_clip = Some(i);
                                    sa.active_clip_name = Some(name.clone());
                                    sa.current_time = 0.0;
                                }
                            }
                        });
                });
            }

            property_row(ui, "Playing", |ui| {
                ui.checkbox(&mut sa.playing, "");
            });
            property_row(ui, "Loop", |ui| {
                ui.checkbox(&mut sa.loop_animation, "");
            });
            property_row(ui, "Speed", |ui| {
                ui.add(DragValue::new(&mut sa.speed).speed(0.05).range(0.1..=10.0));
            });
            if sa.current_time > 0.0 {
                property_row(ui, "Time", |ui| {
                    ui.label(format!("{:.2}s", sa.current_time));
                });
            }

            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("Clips: {}", sa.clip_names.len()))
                .color(theme::TEXT_DISABLED).size(11.0));
        });
        if remove_skel {
            self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
            self.world.remove_skeletal_animator(eid);
        }
    }
}
