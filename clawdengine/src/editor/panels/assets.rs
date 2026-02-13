use egui::Color32;

use crate::editor::context::{AssetEntry, AssetModal};
use crate::editor::layout::EditorTabViewer;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_assets(&mut self, ui: &mut egui::Ui) {
        // Auto-refresh assets every frame (cheap directory listing)
        self.editor_ctx.refresh_assets();

        use crate::editor::theme;

        // Header with refresh
        ui.horizontal(|ui| {
            ui.strong("Assets");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("\u{21BB}").on_hover_text("Refresh").clicked() {
                    self.editor_ctx.refresh_assets();
                }
            });
        });

        // Search bar
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("\u{1F50D}").size(12.0).color(theme::TEXT_DISABLED));
            ui.add(
                egui::TextEdit::singleline(&mut self.editor_ctx.asset_search)
                    .hint_text("Search assets...")
                    .desired_width(ui.available_width()),
            );
        });

        // Breadcrumb navigation (styled with > separators)
        let path = self.editor_ctx.asset_current_dir.clone();
        let components: Vec<_> = path.components().collect();
        ui.horizontal_wrapped(|ui| {
            for (i, component) in components.iter().enumerate() {
                let label = component.as_os_str().to_string_lossy();
                if i > 0 {
                    ui.label(egui::RichText::new("\u{203A}").color(theme::TEXT_DISABLED));
                }
                let is_last = i == components.len() - 1;
                let text = if is_last {
                    egui::RichText::new(label.as_ref()).strong().color(theme::TEXT_PRIMARY)
                } else {
                    egui::RichText::new(label.as_ref()).color(theme::TEXT_SECONDARY)
                };
                if ui.small_button(text).clicked() {
                    let new_path: std::path::PathBuf = components[..=i].iter().collect();
                    self.editor_ctx.asset_current_dir = new_path;
                    self.editor_ctx.refresh_assets();
                }
            }
        });
        ui.separator();

        // Lazy-load icons on first frame
        if self.editor_ctx.icons.is_none() {
            let icons = crate::editor::icons::EditorIcons::load(ui.ctx());
            self.editor_ctx.icons = Some(icons);
        }

        let asset_search = self.editor_ctx.asset_search.to_lowercase();

        // Grid content area (icon view like Unity/Finder)
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.editor_ctx.asset_entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("\u{1F4C2}").size(32.0).color(theme::TEXT_DISABLED));
                    ui.weak("Empty folder");
                });
            }

            let entries: Vec<_> = self.editor_ctx.asset_entries.clone().into_iter()
                .filter(|e| {
                    if asset_search.is_empty() { return true; }
                    let name = match e {
                        AssetEntry::Folder(n) | AssetEntry::File(n) => n,
                    };
                    name.to_lowercase().contains(&asset_search)
                })
                .collect();
            let item_w: f32 = 72.0;
            let item_h: f32 = 80.0;
            let icon_sz: f32 = 48.0;

            // Get icon texture IDs
            let (folder_tex, script_tex, mesh_tex, file_tex) =
                if let Some(ref icons) = self.editor_ctx.icons {
                    (icons.folder.id(), icons.script.id(), icons.mesh.id(), icons.file.id())
                } else {
                    return;
                };

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                for entry in &entries {
                    let (name, tex_id, is_folder) = match entry {
                        AssetEntry::Folder(n) => (n.as_str(), folder_tex, true),
                        AssetEntry::File(n) if n.ends_with(".rs") => (n.as_str(), script_tex, false),
                        AssetEntry::File(n) if n.ends_with(".obj") || n.ends_with(".glb") || n.ends_with(".gltf") => (n.as_str(), mesh_tex, false),
                        AssetEntry::File(n) => (n.as_str(), file_tex, false),
                    };

                    let is_mesh = !is_folder && (name.ends_with(".obj") || name.ends_with(".glb") || name.ends_with(".gltf"));
                    let sense = if is_mesh {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::click()
                    };
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(item_w, item_h),
                        sense,
                    );

                    let painter = ui.painter();

                    // Hover highlight (stronger)
                    if resp.hovered() {
                        painter.rect_filled(rect, 6.0, Color32::from_white_alpha(25));
                    }

                    // Icon image
                    let icon_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.center().x, rect.top() + icon_sz / 2.0 + 2.0),
                        egui::vec2(icon_sz, icon_sz),
                    );
                    painter.image(
                        tex_id,
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Name below icon (truncated + tooltip)
                    let truncated = name.chars().count() > 10;
                    let display_name = if truncated {
                        let trunc: String = name.chars().take(8).collect();
                        format!("{}...", trunc)
                    } else {
                        name.to_string()
                    };
                    let name_pos = egui::pos2(rect.center().x, icon_rect.bottom() + 3.0);
                    painter.text(
                        name_pos,
                        egui::Align2::CENTER_TOP,
                        &display_name,
                        egui::FontId::proportional(11.0),
                        theme::TEXT_SECONDARY,
                    );
                    if truncated {
                        resp.clone().on_hover_text(name);
                    }

                    // Interactions
                    let full_path = self.editor_ctx.asset_current_dir.join(name);
                    resp.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            self.editor_ctx.pending_delete_asset = Some(full_path.clone());
                            ui.close();
                        }
                    });

                    // Drag-and-drop: set payload for mesh files (before move of full_path)
                    if is_mesh {
                        resp.dnd_set_drag_payload(full_path.to_string_lossy().into_owned());
                    }

                    if is_folder {
                        if resp.double_clicked() {
                            self.editor_ctx.asset_current_dir = full_path;
                            self.editor_ctx.refresh_assets();
                        }
                    } else if resp.double_clicked() && is_mesh {
                        self.editor_ctx.pending_load_asset = Some(
                            full_path.to_string_lossy().into_owned(),
                        );
                    }
                }
            });

            // Remaining space for right-click on empty area
            let remaining = ui.available_size();
            let (_rect, response) = ui.allocate_exact_size(
                egui::vec2(remaining.x.max(1.0), remaining.y.max(30.0)),
                egui::Sense::click(),
            );
            response.context_menu(|ui| {
                if ui.button("New Folder").clicked() {
                    self.editor_ctx.asset_modal = Some(AssetModal::NewFolder {
                        name: String::new(),
                    });
                    ui.close();
                }
                if ui.button("New Script").clicked() {
                    self.editor_ctx.asset_modal = Some(AssetModal::NewScript {
                        name: String::new(),
                    });
                    ui.close();
                }
                if ui.button("New Scene").clicked() {
                    self.editor_ctx.asset_modal = Some(AssetModal::NewScene {
                        name: "New Scene".to_string(),
                    });
                    ui.close();
                }
            });
        });
    }
}
