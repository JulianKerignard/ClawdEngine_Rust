use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

use super::context::{AssetModal, EditorContext, EditorTool};
use super::theme;

pub fn show_header(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    egui::TopBottomPanel::top("header")
        .exact_height(38.0)
        .resizable(false)
        .frame(
            Frame::NONE
                .fill(theme::BG_MANTLE)
                .inner_margin(Margin::symmetric(12, 0))
                .stroke(Stroke::new(1.0, theme::BG_SURFACE0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Logo with accent color
                ui.label(
                    egui::RichText::new("ClawdEngine")
                        .color(theme::ACCENT)
                        .strong()
                        .size(15.0),
                );

                // Scene name
                if !editor_ctx.scene_name.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("  \u{2022}  {}", editor_ctx.scene_name))
                            .color(theme::TEXT_DISABLED)
                            .size(12.0),
                    );
                }

                ui.add_space(12.0);

                // Vertical separator
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(1.0, 20.0), egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, theme::BG_SURFACE0);

                ui.add_space(8.0);

                // File menu
                ui.menu_button(
                    egui::RichText::new("File").color(theme::TEXT_SECONDARY),
                    |ui| {
                        if ui.button("New Scene").clicked() {
                            editor_ctx.asset_modal = Some(AssetModal::NewScene { name: "New Scene".to_string() });
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Save  (Ctrl+S)").clicked() {
                            editor_ctx.pending_save_scene = Some(editor_ctx.scene_name.clone());
                            ui.close();
                        }
                        ui.separator();
                        if let Ok(entries) = std::fs::read_dir("assets/scenes") {
                            let mut scenes: Vec<String> = entries.flatten()
                                .filter_map(|e| {
                                    let p = e.path();
                                    if p.extension().is_some_and(|ext| ext == "ron") {
                                        p.file_stem().map(|s| s.to_string_lossy().into_owned())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            scenes.sort();
                            if scenes.is_empty() {
                                ui.label(egui::RichText::new("No saved scenes").color(theme::TEXT_DISABLED));
                            } else {
                                ui.label(egui::RichText::new("Load scene:").color(theme::TEXT_DISABLED).small());
                                for scene in scenes {
                                    if ui.button(&scene).clicked() {
                                        editor_ctx.pending_load_scene = Some(scene);
                                        ui.close();
                                    }
                                }
                            }
                        }
                    },
                );

                ui.add_space(4.0);

                // View menu
                ui.menu_button(
                    egui::RichText::new("View").color(theme::TEXT_SECONDARY),
                    |ui| {
                        let grid_label = if editor_ctx.show_grid { "\u{2611} Grid" } else { "\u{2610} Grid" };
                        if ui.button(grid_label).clicked() {
                            editor_ctx.show_grid = !editor_ctx.show_grid;
                            ui.close();
                        }
                        let stats_label = if editor_ctx.show_stats_overlay { "\u{2611} Stats Overlay" } else { "\u{2610} Stats Overlay" };
                        if ui.button(stats_label).clicked() {
                            editor_ctx.show_stats_overlay = !editor_ctx.show_stats_overlay;
                            ui.close();
                        }
                    },
                );

                ui.add_space(8.0);

                // Vertical separator
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(1.0, 20.0), egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, theme::BG_SURFACE0);

                ui.add_space(8.0);

                // Tool group in a dark frame
                Frame::NONE
                    .fill(theme::BG_CRUST)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let tools = [
                                (EditorTool::Select, "\u{25C7}", "Select (Q)"),
                                (EditorTool::Move,   "\u{271A}", "Move (W)"),
                                (EditorTool::Rotate, "\u{21BB}", "Rotate (E)"),
                                (EditorTool::Scale,  "\u{2922}", "Scale (R)"),
                            ];
                            for (tool, icon, tooltip) in &tools {
                                let active = editor_ctx.active_tool == *tool;
                                let text = if active {
                                    egui::RichText::new(*icon).color(Color32::WHITE).size(15.0)
                                } else {
                                    egui::RichText::new(*icon).color(theme::TEXT_DISABLED).size(15.0)
                                };
                                let btn = if active {
                                    egui::Button::new(text)
                                        .fill(theme::ACCENT)
                                        .corner_radius(CornerRadius::same(4))
                                } else {
                                    egui::Button::new(text)
                                        .fill(Color32::TRANSPARENT)
                                        .corner_radius(CornerRadius::same(4))
                                };
                                if ui.add(btn).on_hover_text(*tooltip).clicked() {
                                    editor_ctx.active_tool = *tool;
                                }
                            }
                        });
                    });

                ui.add_space(8.0);

                // Undo/Redo buttons
                {
                    let can_undo = editor_ctx.undo_stack.can_undo();
                    let can_redo = editor_ctx.undo_stack.can_redo();
                    let undo_color = if can_undo { theme::TEXT_SECONDARY } else { theme::TEXT_DISABLED.gamma_multiply(0.5) };
                    let redo_color = if can_redo { theme::TEXT_SECONDARY } else { theme::TEXT_DISABLED.gamma_multiply(0.5) };

                    let undo_btn = ui.add(
                        egui::Button::new(egui::RichText::new("\u{21B6}").color(undo_color).size(15.0))
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(CornerRadius::same(4)),
                    );
                    if undo_btn.on_hover_text("Undo (Cmd+Z)").clicked() && can_undo {
                        editor_ctx.pending_undo = true;
                    }
                    let redo_btn = ui.add(
                        egui::Button::new(egui::RichText::new("\u{21B7}").color(redo_color).size(15.0))
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(CornerRadius::same(4)),
                    );
                    if redo_btn.on_hover_text("Redo (Cmd+Shift+Z)").clicked() && can_redo {
                        editor_ctx.pending_redo = true;
                    }
                }

                // Center: Play/Stop button
                let remaining = ui.available_width();
                ui.add_space((remaining * 0.5 - 60.0).max(0.0));

                if editor_ctx.play_mode {
                    let btn = egui::Button::new(
                        egui::RichText::new("\u{23F9}  Stop")
                            .color(Color32::WHITE)
                            .size(14.0),
                    )
                    .fill(theme::ERROR)
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).on_hover_text("Stop (Space)").clicked() {
                        editor_ctx.pending_play_toggle = Some(false);
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(" PLAYING ")
                            .background_color(theme::ERROR.gamma_multiply(0.25))
                            .color(theme::ERROR)
                            .small()
                            .strong(),
                    );
                } else {
                    let btn = egui::Button::new(
                        egui::RichText::new("\u{25B6}  Play")
                            .color(theme::BG_CRUST)
                            .size(14.0),
                    )
                    .fill(theme::SUCCESS)
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).on_hover_text("Play (Space)").clicked() {
                        editor_ctx.pending_play_toggle = Some(true);
                    }
                }

                // Fullscreen game button
                ui.add_space(6.0);
                let fs_btn = egui::Button::new(
                    egui::RichText::new("\u{26F6}")
                        .color(theme::TEXT_SECONDARY)
                        .size(14.0),
                )
                .fill(Color32::TRANSPARENT)
                .corner_radius(CornerRadius::same(4));
                if ui.add(fs_btn).on_hover_text("Fullscreen Game (F5)").clicked() {
                    if !editor_ctx.play_mode {
                        editor_ctx.pending_play_toggle = Some(true);
                    }
                    editor_ctx.fullscreen_game = true;
                }

                // Right side: feedback + FPS
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if let Some(ref msg) = editor_ctx.loading_status {
                            ui.label(
                                egui::RichText::new(msg.as_str())
                                    .color(theme::ACCENT)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                        }
                        if let Some((ref msg, _)) = editor_ctx.save_feedback {
                            ui.label(
                                egui::RichText::new(msg.as_str())
                                    .color(theme::SUCCESS)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                        }

                        let (err_count, warn_count, _) = editor_ctx.log_buffer.counts();
                        if err_count > 0 {
                            ui.label(
                                egui::RichText::new(format!("{} err", err_count))
                                    .color(theme::ERROR)
                                    .size(11.0)
                                    .monospace(),
                            );
                            ui.add_space(4.0);
                        }
                        if warn_count > 0 {
                            ui.label(
                                egui::RichText::new(format!("{} warn", warn_count))
                                    .color(theme::WARNING)
                                    .size(11.0)
                                    .monospace(),
                            );
                            ui.add_space(4.0);
                        }

                        ui.label(
                            egui::RichText::new(format!(
                                "{:.0} FPS | {} entities",
                                editor_ctx.fps, editor_ctx.entity_count
                            ))
                            .color(theme::TEXT_DISABLED)
                            .monospace(),
                        );
                    },
                );
            });
        });
}
