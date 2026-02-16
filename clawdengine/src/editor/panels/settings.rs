use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;
use crate::assets::settings;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.add_space(4.0);
            ui.strong("Project Settings");
            ui.separator();

            let mut changed = false;

            // ---- Rendering ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{1F3A8} Rendering").color(theme::TEXT_PRIMARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("MSAA").color(theme::TEXT_SECONDARY));
                    let current = self.editor_ctx.project_settings.rendering.msaa_samples;
                    egui::ComboBox::from_id_salt("msaa")
                        .selected_text(format!("{}x", current))
                        .show_ui(ui, |ui| {
                            for val in [1, 2, 4] {
                                if ui.selectable_value(
                                    &mut self.editor_ctx.project_settings.rendering.msaa_samples,
                                    val,
                                    format!("{}x", val),
                                ).changed() {
                                    changed = true;
                                }
                            }
                        });
                    ui.label(egui::RichText::new("(restart)").color(theme::WARNING).small());
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Anisotropy").color(theme::TEXT_SECONDARY));
                    let current = self.editor_ctx.project_settings.rendering.anisotropy;
                    egui::ComboBox::from_id_salt("aniso")
                        .selected_text(format!("{}x", current))
                        .show_ui(ui, |ui| {
                            for val in [1, 4, 8, 16] {
                                if ui.selectable_value(
                                    &mut self.editor_ctx.project_settings.rendering.anisotropy,
                                    val,
                                    format!("{}x", val),
                                ).changed() {
                                    changed = true;
                                }
                            }
                        });
                    ui.label(egui::RichText::new("(restart)").color(theme::WARNING).small());
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Window").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.rendering.window_width)
                        .range(640..=3840)
                        .prefix("W: ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.rendering.window_height)
                        .range(480..=2160)
                        .prefix("H: ")).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Shadows ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{1F311} Shadows").color(theme::TEXT_PRIMARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Resolution").color(theme::TEXT_SECONDARY));
                    let current = self.editor_ctx.project_settings.shadows.map_resolution;
                    egui::ComboBox::from_id_salt("shadow_res")
                        .selected_text(format!("{}", current))
                        .show_ui(ui, |ui| {
                            for val in [512, 1024, 2048, 4096] {
                                if ui.selectable_value(
                                    &mut self.editor_ctx.project_settings.shadows.map_resolution,
                                    val,
                                    format!("{}", val),
                                ).changed() {
                                    changed = true;
                                }
                            }
                        });
                    ui.label(egui::RichText::new("(restart)").color(theme::WARNING).small());
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Distance").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.shadows.light_distance)
                        .speed(0.5).range(1.0..=200.0)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Ortho Size").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.shadows.ortho_size)
                        .speed(0.5).range(1.0..=200.0)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Near").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.shadows.near_plane)
                        .speed(0.01).range(0.001..=10.0)).changed();
                    ui.label(egui::RichText::new("Far").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.shadows.far_plane)
                        .speed(0.5).range(10.0..=500.0)).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Lighting ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{2600} Lighting").color(theme::TEXT_PRIMARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Ambient").color(theme::TEXT_SECONDARY));
                    changed |= ui.color_edit_button_rgba_unmultiplied(
                        &mut self.editor_ctx.project_settings.lighting.ambient_color,
                    ).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Camera ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{1F3A5} Camera").color(theme::TEXT_PRIMARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("FOV").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::Slider::new(
                        &mut self.editor_ctx.project_settings.camera.fov,
                        10.0..=120.0,
                    ).suffix("°")).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Near").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.camera.near_clip)
                        .speed(0.01).range(0.001..=10.0)).changed();
                    ui.label(egui::RichText::new("Far").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.camera.far_clip)
                        .speed(1.0).range(10.0..=10000.0)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Rotate Sens.").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.camera.rotate_sensitivity)
                        .speed(0.0005).range(0.0001..=0.05)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Pan Sens.").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.camera.pan_sensitivity)
                        .speed(0.0005).range(0.0001..=0.05)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Zoom Speed").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.camera.zoom_speed)
                        .speed(0.05).range(0.01..=5.0)).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Grid ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{25A6} Grid").color(theme::TEXT_PRIMARY),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Half Size").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.grid.half_size)
                        .range(1..=100)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Color").color(theme::TEXT_SECONDARY));
                    changed |= ui.color_edit_button_rgba_unmultiplied(
                        &mut self.editor_ctx.project_settings.grid.color,
                    ).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Physics ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{1F30D} Physics").color(theme::TEXT_PRIMARY),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Gravity").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.physics.gravity)
                        .speed(0.1).range(0.0..=100.0)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Ground Y").color(theme::TEXT_SECONDARY));
                    changed |= ui.add(egui::DragValue::new(&mut self.editor_ctx.project_settings.physics.ground_y)
                        .speed(0.1).range(-100.0..=100.0)).changed();
                });
            });

            ui.add_space(4.0);

            // ---- Build ----
            egui::CollapsingHeader::new(
                egui::RichText::new("\u{1F4E6} Build").color(theme::TEXT_PRIMARY),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("App Name").color(theme::TEXT_SECONDARY));
                    changed |= ui.text_edit_singleline(
                        &mut self.editor_ctx.project_settings.build.app_name,
                    ).changed();
                });
                ui.label(egui::RichText::new("Empty = use project name").color(theme::TEXT_DISABLED).small());

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Bundle ID").color(theme::TEXT_SECONDARY));
                    changed |= ui.text_edit_singleline(
                        &mut self.editor_ctx.project_settings.build.bundle_id_prefix,
                    ).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Version").color(theme::TEXT_SECONDARY));
                    changed |= ui.text_edit_singleline(
                        &mut self.editor_ctx.project_settings.build.version,
                    ).changed();
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Min macOS").color(theme::TEXT_SECONDARY));
                    changed |= ui.text_edit_singleline(
                        &mut self.editor_ctx.project_settings.build.min_macos_version,
                    ).changed();
                });
            });

            ui.add_space(8.0);
            ui.separator();

            // ---- Buttons ----
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let _ = settings::save_settings(&self.editor_ctx.project_settings);
                    self.editor_ctx.save_feedback = Some(("Settings saved".to_string(), 2.0));
                }
                if ui.button("Reset to Defaults").clicked() {
                    self.editor_ctx.project_settings = settings::ProjectSettings::default();
                    changed = true;
                }
            });

            if changed {
                self.editor_ctx.settings_dirty = true;
            }
        });
    }
}
