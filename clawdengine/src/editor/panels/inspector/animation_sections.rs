use egui::{Color32, ComboBox, DragValue};

use crate::core::AnimatorParameter;
use crate::editor::layout::{EditorTabViewer, component_section, property_row};
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_animator(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_animator(eid).is_none() {
            return;
        }
        const ANIM_ACCENT: Color32 = Color32::from_rgb(0xA6, 0xE3, 0xA1);
        let remove_anim = component_section(ui, "animator", "An", "Animator", ANIM_ACCENT, true, |ui| {
            let Some(anim) = self.world.get_animator_mut(eid) else { return; };

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
            let Some(sa) = self.world.get_skeletal_animator_mut(eid) else { return; };

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

    pub(super) fn inspector_animator_controller(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        // Only show if the entity has a SkeletalAnimator with a controller assigned
        let has_controller = self.world.get_skeletal_animator(eid)
            .map(|sa| sa.controller_id.is_some())
            .unwrap_or(false);

        if !has_controller { return; }

        const CTRL_ACCENT: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7); // mauve

        component_section(ui, "animator_controller", "AC", "Animator Controller", CTRL_ACCENT, false, |ui| {
            // ---- Read-only info (immutable borrow) ----
            let (ctrl_name, state_name, is_blending, blend_progress, prev_state_name) = {
                let sa = self.world.get_skeletal_animator(eid).unwrap();
                let ctrl_name = sa.controller_name.clone().unwrap_or_else(|| "(none)".into());
                let mut state_name = "(none)".to_string();
                let mut is_blending = false;
                let mut blend_progress = 0.0f32;
                let mut prev_state_name = String::new();

                if let Some(ref cs) = sa.controller_state {
                    state_name = format!("State {}", cs.current_state);
                    is_blending = cs.is_blending;
                    blend_progress = cs.blend_progress;
                    if let Some(ps) = cs.previous_state {
                        prev_state_name = format!("State {}", ps);
                    }
                }
                (ctrl_name, state_name, is_blending, blend_progress, prev_state_name)
            };

            property_row(ui, "Controller", |ui| {
                ui.label(&ctrl_name);
            });
            property_row(ui, "State", |ui| {
                ui.label(egui::RichText::new(&state_name).strong());
            });

            if is_blending {
                property_row(ui, "Blend", |ui| {
                    ui.label(format!("{} \u{2192} {}", prev_state_name, state_name));
                });
                ui.add(egui::ProgressBar::new(blend_progress).show_percentage());
            }

            // ---- Parameters section ----
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Parameters").color(theme::TEXT_DISABLED).small());

            // Collect parameter names to avoid borrow issues
            let mut param_names: Vec<String> = {
                let sa = self.world.get_skeletal_animator(eid).unwrap();
                sa.controller_state.as_ref()
                    .map(|cs| cs.parameters.keys().cloned().collect())
                    .unwrap_or_default()
            };
            param_names.sort();

            if param_names.is_empty() {
                ui.label(egui::RichText::new("No parameters").color(theme::TEXT_DISABLED).size(11.0));
            }

            for name in &param_names {
                // Read current value (immutable borrow, then drop)
                let param_value = {
                    let sa = self.world.get_skeletal_animator(eid).unwrap();
                    sa.controller_state.as_ref()
                        .and_then(|cs| cs.parameters.get(name).cloned())
                };

                if let Some(param) = param_value {
                    match param {
                        AnimatorParameter::Float(mut v) => {
                            property_row(ui, name, |ui| {
                                if ui.add(DragValue::new(&mut v).speed(0.05)).changed() {
                                    if let Some(sa) = self.world.get_skeletal_animator_mut(eid) {
                                        if let Some(ref mut cs) = sa.controller_state {
                                            cs.set_float(name, v);
                                        }
                                    }
                                }
                            });
                        }
                        AnimatorParameter::Int(mut v) => {
                            property_row(ui, name, |ui| {
                                if ui.add(DragValue::new(&mut v).speed(0.1)).changed() {
                                    if let Some(sa) = self.world.get_skeletal_animator_mut(eid) {
                                        if let Some(ref mut cs) = sa.controller_state {
                                            cs.set_int(name, v);
                                        }
                                    }
                                }
                            });
                        }
                        AnimatorParameter::Bool(mut v) => {
                            property_row(ui, name, |ui| {
                                if ui.checkbox(&mut v, "").changed() {
                                    if let Some(sa) = self.world.get_skeletal_animator_mut(eid) {
                                        if let Some(ref mut cs) = sa.controller_state {
                                            cs.set_bool(name, v);
                                        }
                                    }
                                }
                            });
                        }
                        AnimatorParameter::Trigger(_) => {
                            property_row(ui, name, |ui| {
                                if ui.button("Fire").clicked() {
                                    if let Some(sa) = self.world.get_skeletal_animator_mut(eid) {
                                        if let Some(ref mut cs) = sa.controller_state {
                                            cs.set_trigger(name);
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });
    }

    pub(super) fn inspector_bone(&mut self, ui: &mut egui::Ui) {
        let Some((eid, bone_idx)) = self.editor_ctx.selected_bone else { return };

        let anim = match self.world.get_skeletal_animator(eid) {
            Some(a) => a,
            None => return,
        };
        let skel_id = match anim.skeleton_id {
            Some(id) => id,
            None => return,
        };
        let skeleton = match self.skeleton_store.get(skel_id) {
            Some(s) => s,
            None => return,
        };
        if bone_idx >= skeleton.bones.len() { return; }

        let bone = &skeleton.bones[bone_idx];
        let local_pose = anim.current_local_poses.get(bone_idx).copied();

        const BONE_ACCENT: Color32 = Color32::from_rgb(0xF0, 0xC0, 0x40);
        component_section(ui, "bone_info", "\u{1F9B4}", "Bone", BONE_ACCENT, false, |ui| {
            property_row(ui, "Name", |ui| {
                ui.label(egui::RichText::new(&bone.name).strong());
            });

            property_row(ui, "Index", |ui| {
                ui.label(format!("{}", bone_idx));
            });

            property_row(ui, "Parent", |ui| {
                let parent_name = bone.parent
                    .and_then(|pi| skeleton.bones.get(pi))
                    .map(|b| b.name.as_str())
                    .unwrap_or("(root)");
                ui.label(parent_name);
            });

            property_row(ui, "Children", |ui| {
                ui.label(format!("{}", bone.children.len()));
            });

            if let Some(pose) = local_pose {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Local Transform").color(theme::TEXT_DISABLED).size(11.0));

                property_row(ui, "Position", |ui| {
                    ui.label(format!("{:.3}, {:.3}, {:.3}", pose.position.x, pose.position.y, pose.position.z));
                });

                let (axis, angle) = pose.rotation.to_axis_angle();
                property_row(ui, "Rotation", |ui| {
                    ui.label(format!("{:.1}\u{00B0}", angle.to_degrees()));
                    ui.label(egui::RichText::new(format!("({:.2}, {:.2}, {:.2})", axis.x, axis.y, axis.z))
                        .color(theme::TEXT_DISABLED).size(10.0));
                });

                property_row(ui, "Scale", |ui| {
                    ui.label(format!("{:.3}, {:.3}, {:.3}", pose.scale.x, pose.scale.y, pose.scale.z));
                });
            }
        });
    }
}
