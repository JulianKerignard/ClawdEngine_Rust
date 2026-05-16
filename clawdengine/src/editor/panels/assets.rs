use egui::{Color32, CornerRadius, Stroke, StrokeKind};

use crate::editor::context::{AssetEntry, AssetModal};
use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

/// Persistent selection stored in egui memory (avoids touching EditorContext).
const ASSET_SEL_ID: &str = "asset_panel_selected";

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_assets(&mut self, ui: &mut egui::Ui) {
        // Auto-refresh assets every frame (cheap directory listing)
        self.editor_ctx.refresh_assets();

        // ---- Toolbar ----
        ui.horizontal(|ui| {
            // "+ Create" button
            let create_btn = egui::Button::new(
                egui::RichText::new("+ Create")
                    .color(theme::TEXT_PRIMARY)
                    .size(11.5),
            )
            .fill(theme::BG_SURFACE0)
            .corner_radius(CornerRadius::same(4))
            .stroke(Stroke::new(1.0, theme::BG_SURFACE1));

            if ui.add(create_btn).clicked() {
                // Open context menu equivalent — show modal for new items
                self.editor_ctx.asset_modal = Some(AssetModal::NewFolder {
                    name: String::new(),
                });
            }

            ui.add_space(4.0);

            // Refresh button
            let refresh_btn = egui::Button::new(
                egui::RichText::new("\u{21BB}")
                    .color(theme::TEXT_SECONDARY)
                    .size(12.0),
            )
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE);

            if ui.add(refresh_btn)
                .on_hover_text("Refresh")
                .clicked()
            {
                self.editor_ctx.refresh_assets();
            }

            ui.add_space(4.0);

            // Search bar
            let search_icon = egui::RichText::new("\u{1F50D}")
                .size(11.0)
                .color(theme::TEXT_DISABLED);
            ui.label(search_icon);

            ui.add(
                egui::TextEdit::singleline(&mut self.editor_ctx.asset_search)
                    .hint_text("Search assets...")
                    .desired_width(140.0)
                    .font(egui::FontId::proportional(11.5)),
            );
        });

        ui.add_space(2.0);

        // ---- Breadcrumb navigation ----
        // Separator line above breadcrumbs
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, theme::BG_SURFACE0),
        );

        let path = self.editor_ctx.asset_current_dir.clone();
        let components: Vec<_> = path.components().collect();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (i, component) in components.iter().enumerate() {
                let label = component.as_os_str().to_string_lossy();
                let is_last = i == components.len() - 1;

                if i > 0 {
                    // Separator › (U+203A)
                    ui.label(
                        egui::RichText::new("\u{203A}")
                            .color(theme::TEXT_DISABLED)
                            .size(11.0),
                    );
                }

                if is_last {
                    // Current segment: TEXT_PRIMARY, not clickable (just a label)
                    ui.label(
                        egui::RichText::new(label.as_ref())
                            .color(theme::TEXT_PRIMARY)
                            .size(11.5),
                    );
                } else {
                    // Parent segments: TEXT_DISABLED, clickable
                    let crumb_resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(label.as_ref())
                                .color(theme::TEXT_DISABLED)
                                .size(11.5),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if crumb_resp.hovered() {
                        // Subtle underline on hover to show clickability
                        let galley_rect = crumb_resp.rect;
                        ui.painter().hline(
                            galley_rect.x_range(),
                            galley_rect.bottom() - 1.0,
                            Stroke::new(1.0, theme::TEXT_DISABLED),
                        );
                    }
                    if crumb_resp.clicked() {
                        let new_path: std::path::PathBuf = components[..=i].iter().collect();
                        self.editor_ctx.asset_current_dir = new_path;
                        self.editor_ctx.refresh_assets();
                    }
                }
            }
        });

        ui.add_space(2.0);
        ui.separator();

        // Lazy-load icons on first frame
        if self.editor_ctx.icons.is_none() {
            let icons = crate::editor::icons::EditorIcons::load(ui.ctx());
            self.editor_ctx.icons = Some(icons);
        }

        let asset_search = self.editor_ctx.asset_search.to_lowercase();

        // Read current selection from egui memory
        let sel_id = egui::Id::new(ASSET_SEL_ID);
        let current_sel: Option<String> = ui.ctx().data(|d| d.get_temp(sel_id));

        // Grid content area
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.editor_ctx.asset_entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("\u{1F4C2}")
                            .size(32.0)
                            .color(theme::TEXT_DISABLED),
                    );
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

            let item_w: f32 = 80.0;
            let item_h: f32 = 88.0;
            let thumb_sz: f32 = 56.0;

            // Get icon texture IDs
            let (folder_tex, script_tex, mesh_tex, file_tex) =
                if let Some(ref icons) = self.editor_ctx.icons {
                    (icons.folder.id(), icons.script.id(), icons.mesh.id(), icons.file.id())
                } else {
                    return;
                };

            // New selection to write after the loop
            let mut new_sel: Option<Option<String>> = None;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                for entry in &entries {
                    let (name, is_folder) = match entry {
                        AssetEntry::Folder(n) => (n.as_str(), true),
                        AssetEntry::File(n) => (n.as_str(), false),
                    };

                    let is_mesh = !is_folder
                        && (name.ends_with(".obj")
                            || name.ends_with(".glb")
                            || name.ends_with(".gltf"));

                    let sense = if is_mesh {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::click()
                    };

                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(item_w, item_h),
                        sense,
                    );

                    let is_selected = current_sel.as_deref() == Some(name);

                    let painter = ui.painter();

                    // Item background: selected → ACCENT_SOFT, hover → subtle white
                    if is_selected {
                        painter.rect(
                            rect,
                            CornerRadius::same(4),
                            theme::ACCENT_SOFT,
                            Stroke::new(1.0, theme::ACCENT),
                            StrokeKind::Outside,
                        );
                    } else if resp.hovered() {
                        painter.rect_filled(rect, 4.0, Color32::from_white_alpha(12));
                    }

                    // Thumbnail area (rounded 4px)
                    let thumb_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.center().x, rect.top() + thumb_sz / 2.0 + 4.0),
                        egui::vec2(thumb_sz, thumb_sz),
                    );

                    // Extension label derived from file name
                    let ext = name.rfind('.').map(|i| &name[i + 1..]).unwrap_or("");

                    if is_folder {
                        // Folder: icon image with amber tint
                        painter.rect_filled(
                            thumb_rect,
                            CornerRadius::same(4),
                            theme::BG_SURFACE0,
                        );
                        painter.image(
                            folder_tex,
                            thumb_rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            theme::WARNING,
                        );
                    } else if name.ends_with(".rs") {
                        // Rust file: dark gradient bg + accent glyph + .rs badge
                        painter.rect_filled(
                            thumb_rect,
                            CornerRadius::same(4),
                            theme::BG_SURFACE0,
                        );
                        // Inner dark gradient approximation (two-tone fill)
                        let inner = egui::Rect::from_center_size(
                            thumb_rect.center(),
                            thumb_rect.size() * 0.85,
                        );
                        painter.rect_filled(inner, CornerRadius::same(3), theme::BG_CRUST);

                        // Script icon image tinted with accent
                        painter.image(
                            script_tex,
                            egui::Rect::from_center_size(
                                thumb_rect.center(),
                                egui::vec2(thumb_sz * 0.55, thumb_sz * 0.55),
                            ),
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            theme::ACCENT,
                        );

                        // ".rs" badge bottom-left of thumb
                        let badge_pos = egui::pos2(thumb_rect.left() + 3.0, thumb_rect.bottom() - 3.0);
                        painter.text(
                            badge_pos,
                            egui::Align2::LEFT_BOTTOM,
                            ".rs",
                            egui::FontId::monospace(8.0),
                            theme::ACCENT,
                        );
                    } else if is_mesh {
                        // Mesh: mesh icon tinted with SKY
                        painter.rect_filled(
                            thumb_rect,
                            CornerRadius::same(4),
                            theme::BG_SURFACE0,
                        );
                        painter.image(
                            mesh_tex,
                            thumb_rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            theme::SKY,
                        );
                    } else {
                        // Generic file
                        painter.rect_filled(
                            thumb_rect,
                            CornerRadius::same(4),
                            theme::BG_SURFACE0,
                        );
                        painter.image(
                            file_tex,
                            thumb_rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            Color32::WHITE,
                        );
                        // Extension badge for generic files
                        if !ext.is_empty() {
                            let badge_pos = egui::pos2(
                                thumb_rect.left() + 3.0,
                                thumb_rect.bottom() - 3.0,
                            );
                            painter.text(
                                badge_pos,
                                egui::Align2::LEFT_BOTTOM,
                                &format!(".{}", ext),
                                egui::FontId::monospace(8.0),
                                theme::TEXT_DISABLED,
                            );
                        }
                    }

                    // Name below thumbnail (truncated + tooltip)
                    let truncated = name.chars().count() > 10;
                    let display_name = if truncated {
                        let trunc: String = name.chars().take(8).collect();
                        format!("{}...", trunc)
                    } else {
                        name.to_string()
                    };
                    let name_pos = egui::pos2(rect.center().x, thumb_rect.bottom() + 4.0);
                    painter.text(
                        name_pos,
                        egui::Align2::CENTER_TOP,
                        &display_name,
                        egui::FontId::proportional(10.5),
                        theme::TEXT_SECONDARY,
                    );
                    if truncated {
                        resp.clone().on_hover_text(name);
                    }

                    // Click → select
                    if resp.clicked() {
                        new_sel = Some(Some(name.to_string()));
                    }

                    // Interactions
                    let full_path = self.editor_ctx.asset_current_dir.join(name);
                    resp.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            self.editor_ctx.confirm_delete_asset = Some(full_path.clone());
                            ui.close();
                        }
                    });

                    // Drag-and-drop for mesh files
                    if is_mesh {
                        resp.dnd_set_drag_payload(full_path.to_string_lossy().into_owned());
                    }

                    if is_folder {
                        if resp.double_clicked() {
                            self.editor_ctx.asset_current_dir = full_path;
                            self.editor_ctx.refresh_assets();
                            new_sel = Some(None); // clear selection on navigation
                        }
                    } else if resp.double_clicked() && is_mesh {
                        self.editor_ctx.pending_load_asset =
                            Some(full_path.to_string_lossy().into_owned());
                    }
                }
            });

            // Remaining space: right-click to create new assets, also deselect on click
            let remaining = ui.available_size();
            let (_rect, response) = ui.allocate_exact_size(
                egui::vec2(remaining.x.max(1.0), remaining.y.max(30.0)),
                egui::Sense::click(),
            );
            if response.clicked() {
                new_sel = Some(None);
            }
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

            // Persist selection change
            if let Some(sel) = new_sel {
                ui.ctx().data_mut(|d| d.insert_temp(sel_id, sel));
            }
        });
    }
}
