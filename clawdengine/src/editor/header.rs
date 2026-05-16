use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

use super::context::{AssetModal, EditorContext, EditorTool};
use super::theme;

const ENGINE_VERSION: &str = "0.4.x";

/// Persistent id tracking hover state of the decorative search field so the
/// accent focus ring can be drawn on the next frame.
fn search_hover_id() -> egui::Id {
    egui::Id::new("header_search_hover")
}

pub fn show_header(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    show_menubar(ctx, editor_ctx);
    show_toolbar(ctx, editor_ctx);
}

/// Top row: brand + File/View menus + scene meta. Mirrors the mockup's 32px
/// menubar.
fn show_menubar(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    egui::TopBottomPanel::top("menubar")
        .exact_height(30.0)
        .resizable(false)
        .frame(
            Frame::NONE
                .fill(theme::BG_MANTLE)
                .inner_margin(Margin::symmetric(10, 0))
                .stroke(Stroke::new(1.0, theme::BG_SURFACE0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Square accent logo with "C"
                let (logo_rect, _) =
                    ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(logo_rect, CornerRadius::same(4), theme::ACCENT);
                ui.painter().text(
                    logo_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "C",
                    egui::FontId::proportional(12.0),
                    theme::ON_ACCENT,
                );

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("ClawdEngine")
                        .color(theme::TEXT_PRIMARY)
                        .strong()
                        .size(13.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(ENGINE_VERSION)
                        .color(theme::TEXT_DISABLED)
                        .size(11.0),
                );

                ui.add_space(10.0);
                vertical_separator(ui);
                ui.add_space(8.0);

                // File menu
                ui.menu_button(
                    egui::RichText::new("File").color(theme::TEXT_SECONDARY),
                    |ui| {
                        if ui.button("New Scene").clicked() {
                            editor_ctx.asset_modal =
                                Some(AssetModal::NewScene { name: "New Scene".to_string() });
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Save  (Ctrl+S)").clicked() {
                            editor_ctx.pending_save_scene = Some(editor_ctx.scene_name.clone());
                            ui.close();
                        }
                        ui.separator();
                        if let Ok(entries) = std::fs::read_dir("assets/scenes") {
                            let mut scenes: Vec<String> = entries
                                .flatten()
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
                                ui.label(
                                    egui::RichText::new("No saved scenes")
                                        .color(theme::TEXT_DISABLED),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("Load scene:")
                                        .color(theme::TEXT_DISABLED)
                                        .small(),
                                );
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
                        let grid_label = if editor_ctx.show_grid {
                            "\u{2611} Grid"
                        } else {
                            "\u{2610} Grid"
                        };
                        if ui.button(grid_label).clicked() {
                            editor_ctx.show_grid = !editor_ctx.show_grid;
                            ui.close();
                        }
                        let stats_label = if editor_ctx.show_stats_overlay {
                            "\u{2611} Stats Overlay"
                        } else {
                            "\u{2610} Stats Overlay"
                        };
                        if ui.button(stats_label).clicked() {
                            editor_ctx.show_stats_overlay = !editor_ctx.show_stats_overlay;
                            ui.close();
                        }
                    },
                );

                // Right side: scene meta in mono
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if !editor_ctx.scene_name.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} \u{2022} scene",
                                    editor_ctx.scene_name
                                ))
                                .color(theme::TEXT_DISABLED)
                                .monospace()
                                .size(11.0),
                            );
                        }
                    },
                );
            });
        });
}

/// Second row: tool group, undo/redo, transport (Play/Pause/Step), search,
/// layout pill, live stats. Mirrors the mockup's 40px toolbar.
fn show_toolbar(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    egui::TopBottomPanel::top("toolbar")
        .exact_height(38.0)
        .resizable(false)
        .frame(
            Frame::NONE
                .fill(theme::BG_MANTLE)
                .inner_margin(Margin::symmetric(10, 0))
                .stroke(Stroke::new(1.0, theme::BG_SURFACE0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
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
                                (EditorTool::Move, "\u{271A}", "Move (W)"),
                                (EditorTool::Rotate, "\u{21BB}", "Rotate (E)"),
                                (EditorTool::Scale, "\u{2922}", "Scale (R)"),
                            ];
                            for (tool, icon, tooltip) in &tools {
                                let active = editor_ctx.active_tool == *tool;
                                let (fill, fg) = if active {
                                    (theme::ACCENT, theme::ON_ACCENT)
                                } else {
                                    (Color32::TRANSPARENT, theme::TEXT_DISABLED)
                                };
                                let btn = egui::Button::new(
                                    egui::RichText::new(*icon).color(fg).size(15.0),
                                )
                                .fill(fill)
                                .corner_radius(CornerRadius::same(4));
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
                    let undo_color = if can_undo {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_DISABLED.gamma_multiply(0.5)
                    };
                    let redo_color = if can_redo {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_DISABLED.gamma_multiply(0.5)
                    };

                    let undo_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("\u{21B6}").color(undo_color).size(15.0),
                        )
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(CornerRadius::same(4)),
                    );
                    if undo_btn.on_hover_text("Undo (Cmd+Z)").clicked() && can_undo {
                        editor_ctx.pending_undo = true;
                    }
                    let redo_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("\u{21B7}").color(redo_color).size(15.0),
                        )
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(CornerRadius::same(4)),
                    );
                    if redo_btn.on_hover_text("Redo (Cmd+Shift+Z)").clicked() && can_redo {
                        editor_ctx.pending_redo = true;
                    }
                }

                // Center: transport group (Play/Stop + inert Pause/Step)
                let remaining = ui.available_width();
                ui.add_space((remaining * 0.5 - 70.0).max(0.0));

                Frame::NONE
                    .fill(theme::BG_CRUST)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            if editor_ctx.play_mode {
                                let btn = egui::Button::new(
                                    egui::RichText::new("\u{23F9}  Stop")
                                        .color(Color32::WHITE)
                                        .size(13.0),
                                )
                                .fill(theme::ERROR)
                                .corner_radius(CornerRadius::same(5));
                                if ui.add(btn).on_hover_text("Stop (Space)").clicked() {
                                    editor_ctx.pending_play_toggle = Some(false);
                                }
                                ui.label(
                                    egui::RichText::new(" PLAYING ")
                                        .background_color(theme::ERROR.gamma_multiply(0.25))
                                        .color(theme::ERROR)
                                        .small()
                                        .strong(),
                                );
                            } else {
                                // Manually drawn so hover swaps ACCENT ->
                                // ACCENT_HOVER without painting over the glyph.
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(28.0, 22.0),
                                    egui::Sense::click(),
                                );
                                let play_fill = if resp.hovered() {
                                    theme::ACCENT_HOVER
                                } else {
                                    theme::ACCENT
                                };
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(5),
                                    play_fill,
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{25B6}",
                                    egui::FontId::proportional(13.0),
                                    theme::ON_ACCENT,
                                );
                                if resp.on_hover_text("Play (Space)").clicked() {
                                    editor_ctx.pending_play_toggle = Some(true);
                                }
                            }
                            // Pause / Step are visible but inert for now.
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    egui::RichText::new("\u{23F8}")
                                        .color(theme::TEXT_DISABLED)
                                        .size(13.0),
                                )
                                .fill(Color32::TRANSPARENT)
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .on_disabled_hover_text("Pause (coming soon)");
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    egui::RichText::new("\u{23ED}")
                                        .color(theme::TEXT_DISABLED)
                                        .size(13.0),
                                )
                                .fill(Color32::TRANSPARENT)
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .on_disabled_hover_text("Step (coming soon)");
                        });
                    });

                // Right side: search, layout pill, feedback + stats
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

                        let (err_count, warn_count, _, _) = editor_ctx.log_buffer.counts();
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.0} FPS | {} entities",
                                editor_ctx.fps, editor_ctx.entity_count
                            ))
                            .color(theme::TEXT_DISABLED)
                            .monospace(),
                        );
                        if warn_count > 0 {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("{} warn", warn_count))
                                    .color(theme::WARNING)
                                    .size(11.0)
                                    .monospace(),
                            );
                        }
                        if err_count > 0 {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("{} err", err_count))
                                    .color(theme::ERROR)
                                    .size(11.0)
                                    .monospace(),
                            );
                        }

                        ui.add_space(8.0);

                        // Decorative "Layout · Default" pill
                        Frame::NONE
                            .fill(theme::BG_CRUST)
                            .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Layout \u{2022} Default \u{25BE}")
                                        .color(theme::TEXT_SECONDARY)
                                        .size(11.0),
                                );
                            });

                        ui.add_space(8.0);

                        // Decorative search field with ⌘K hint. Shows an
                        // accent focus ring on hover (mockup parity).
                        let search_hovered = ui
                            .ctx()
                            .data(|d| d.get_temp::<bool>(search_hover_id()))
                            .unwrap_or(false);
                        let search_stroke = if search_hovered {
                            Stroke::new(2.0, theme::ACCENT_RING)
                        } else {
                            Stroke::new(1.0, theme::BG_SURFACE0)
                        };
                        let search_resp = Frame::NONE
                            .fill(theme::BG_CRUST)
                            .stroke(search_stroke)
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("\u{1F50D} Search")
                                            .color(theme::TEXT_DISABLED)
                                            .size(11.0),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new("\u{2318}K")
                                            .color(theme::TEXT_DISABLED)
                                            .monospace()
                                            .size(10.0),
                                    );
                                });
                            })
                            .response;
                        let now_hovered = search_resp
                            .interact(egui::Sense::hover())
                            .hovered();
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(search_hover_id(), now_hovered);
                        });
                    },
                );
            });
        });
}

fn vertical_separator(ui: &mut egui::Ui) {
    let (sep_rect, _) =
        ui.allocate_exact_size(egui::vec2(1.0, 18.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, theme::BG_SURFACE0);
}
