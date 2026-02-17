use egui::Color32;

use crate::editor::context::{AssetEntry, AssetModal};
use crate::editor::layout::EditorTabViewer;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_assets(&mut self, ui: &mut egui::Ui) {
        // Auto-refresh assets every frame (cheap directory listing)
        self.editor_ctx.refresh_assets();

        // Tick highlight timer (fade-out after import)
        if self.editor_ctx.asset_highlight_timer > 0.0 {
            self.editor_ctx.asset_highlight_timer -= ui.input(|i| i.stable_dt);
            if self.editor_ctx.asset_highlight_timer <= 0.0 {
                self.editor_ctx.asset_highlight_timer = 0.0;
                self.editor_ctx.asset_highlight_file = None;
            }
        }

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
        let in_lasso = self.editor_ctx.lasso_origin.is_some();
        let mut sa = egui::ScrollArea::vertical().id_salt("asset_scroll");
        if in_lasso {
            sa = sa.vertical_scroll_offset(self.editor_ctx.lasso_scroll_offset);
        }
        let scroll_out = sa.show(ui, |ui| {
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
            let (folder_tex, script_tex, mesh_tex, file_tex,
                 material_tex, texture_tex, skeleton_tex, clip_tex,
                 audio_tex, scene_tex, prefab_tex) =
                if let Some(ref icons) = self.editor_ctx.icons {
                    (icons.folder.id(), icons.script.id(), icons.mesh.id(), icons.file.id(),
                     icons.material.id(), icons.texture.id(), icons.skeleton.id(), icons.clip.id(),
                     icons.audio.id(), icons.scene.id(), icons.prefab.id())
                } else {
                    return;
                };

            let mut item_rects: Vec<(std::path::PathBuf, egui::Rect)> = Vec::new();

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                for (entry_index, entry) in entries.iter().enumerate() {
                    let (name, tex_id, is_folder) = match entry {
                        AssetEntry::Folder(n) => (n.as_str(), folder_tex, true),
                        AssetEntry::File(n) if n.ends_with(".rs") => (n.as_str(), script_tex, false),
                        AssetEntry::File(n) if n.ends_with(".obj") || n.ends_with(".glb") || n.ends_with(".gltf") || n.ends_with(".fbx") => (n.as_str(), mesh_tex, false),
                        // Sub-asset types with dedicated icons
                        AssetEntry::File(n) if n.ends_with(".skel.ron") => (n.as_str(), skeleton_tex, false),
                        AssetEntry::File(n) if n.ends_with(".clip.ron") => (n.as_str(), clip_tex, false),
                        AssetEntry::File(n) if n.ends_with(".mat.ron") => (n.as_str(), material_tex, false),
                        // Image/texture files
                        AssetEntry::File(n) if n.ends_with(".png") || n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".bmp") || n.ends_with(".tga") => (n.as_str(), texture_tex, false),
                        // Audio files
                        AssetEntry::File(n) if n.ends_with(".wav") || n.ends_with(".mp3") || n.ends_with(".ogg") || n.ends_with(".flac") => (n.as_str(), audio_tex, false),
                        // Prefab files
                        AssetEntry::File(n) if n.ends_with(".prefab.ron") => (n.as_str(), prefab_tex, false),
                        // Scene files
                        AssetEntry::File(n) if n.ends_with(".scene.ron") => (n.as_str(), scene_tex, false),
                        AssetEntry::File(n) => (n.as_str(), file_tex, false),
                    };

                    let is_mesh = !is_folder && (name.ends_with(".obj") || name.ends_with(".glb") || name.ends_with(".gltf") || name.ends_with(".fbx"));
                    let is_mat_file = !is_folder && name.ends_with(".mat.ron");
                    let is_prefab = !is_folder && name.ends_with(".prefab.ron");
                    let is_draggable_asset = is_mesh || is_mat_file || is_prefab || name.ends_with(".clip.ron");
                    // All items sense click_and_drag to prevent OS window drag
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(item_w, item_h),
                        egui::Sense::click_and_drag(),
                    );

                    let painter = ui.painter();

                    // Import highlight (fades out over 3 seconds)
                    let is_highlighted = self.editor_ctx.asset_highlight_file.as_deref() == Some(name);
                    if is_highlighted && self.editor_ctx.asset_highlight_timer > 0.0 {
                        let alpha = (self.editor_ctx.asset_highlight_timer / 3.0).clamp(0.0, 1.0);
                        let c = theme::ACCENT;
                        let highlight_color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 0.3 * 255.0) as u8);
                        painter.rect_filled(rect, 6.0, highlight_color);
                        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 0.6 * 255.0) as u8)), egui::StrokeKind::Outside);
                        ui.ctx().request_repaint();
                    }

                    // Selection highlight (persistent, multi-select)
                    let full_for_select = self.editor_ctx.asset_current_dir.join(name);
                    if !is_folder {
                        item_rects.push((full_for_select.clone(), rect));
                    }
                    let is_selected_asset = self.editor_ctx.is_asset_selected(&full_for_select);
                    if is_selected_asset {
                        let c = theme::ACCENT;
                        painter.rect_filled(rect, 6.0, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 40));
                        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.5, c), egui::StrokeKind::Outside);
                    }

                    // Hover highlight
                    if resp.hovered() && !is_selected_asset {
                        painter.rect_filled(rect, 6.0, Color32::from_white_alpha(25));
                    }

                    // Icon image (or thumbnail for image files)
                    let icon_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.center().x, rect.top() + icon_sz / 2.0 + 2.0),
                        egui::vec2(icon_sz, icon_sz),
                    );

                    let is_image_file = !is_folder && (name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".bmp") || name.ends_with(".tga"));

                    if is_mat_file {
                        // Material: generate a sphere with the actual albedo color
                        let full_mat_path = self.editor_ctx.asset_current_dir.join(name);
                        let mat_key = format!("mat_{}", full_mat_path.to_string_lossy());
                        let mat_thumb_id = if let Some(handle) = self.editor_ctx.asset_thumbnails.get(&mat_key) {
                            Some(handle.id())
                        } else {
                            // Parse the .mat.ron to get albedo color
                            if let Ok(content) = std::fs::read_to_string(&full_mat_path) {
                                if let Ok(mat) = ron::from_str::<crate::assets::extracted_assets::ExtractedMaterial>(&content) {
                                    let sphere = crate::editor::icons::generate_material_sphere(
                                        mat.albedo[0] * 255.0,
                                        mat.albedo[1] * 255.0,
                                        mat.albedo[2] * 255.0,
                                    );
                                    let size = [sphere.width() as usize, sphere.height() as usize];
                                    let pixels: Vec<egui::Color32> = sphere.pixels()
                                        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                                        .collect();
                                    let color_img = egui::ColorImage { size, source_size: egui::Vec2::new(size[0] as f32, size[1] as f32), pixels };
                                    let handle = ui.ctx().load_texture(
                                        format!("mat_sphere_{}", mat_key),
                                        color_img,
                                        egui::TextureOptions::LINEAR,
                                    );
                                    let tid = handle.id();
                                    self.editor_ctx.asset_thumbnails.insert(mat_key, handle);
                                    Some(tid)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        if let Some(tid) = mat_thumb_id {
                            painter.image(
                                tid,
                                icon_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            // Fallback to default material icon
                            painter.image(
                                tex_id,
                                icon_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                    } else if is_image_file {
                        // Try to load/show thumbnail for image files
                        let full_thumb_path = self.editor_ctx.asset_current_dir.join(name);
                        let thumb_key = full_thumb_path.to_string_lossy().to_string();
                        let thumb_id = if let Some(handle) = self.editor_ctx.asset_thumbnails.get(&thumb_key) {
                            Some(handle.id())
                        } else {
                            // Load thumbnail on demand
                            if let Ok(dyn_img) = image::open(&full_thumb_path) {
                                let thumb = dyn_img.thumbnail(icon_sz as u32, icon_sz as u32).into_rgba8();
                                let size = [thumb.width() as usize, thumb.height() as usize];
                                let pixels: Vec<egui::Color32> = thumb.pixels()
                                    .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                                    .collect();
                                let color_img = egui::ColorImage { size, source_size: egui::Vec2::new(size[0] as f32, size[1] as f32), pixels };
                                let handle = ui.ctx().load_texture(
                                    format!("thumb_{}", thumb_key),
                                    color_img,
                                    egui::TextureOptions::LINEAR,
                                );
                                let tid = handle.id();
                                self.editor_ctx.asset_thumbnails.insert(thumb_key, handle);
                                Some(tid)
                            } else {
                                None
                            }
                        };

                        if let Some(tid) = thumb_id {
                            // Draw thumbnail with a subtle border
                            painter.rect_filled(icon_rect.expand(1.0), 3.0, Color32::from_gray(50));
                            painter.image(
                                tid,
                                icon_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            // Fallback to texture icon
                            painter.image(
                                tex_id,
                                icon_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                    } else {
                        painter.image(
                            tex_id,
                            icon_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }

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
                        let sel_count = self.editor_ctx.selected_assets.len();
                        if sel_count > 1 && self.editor_ctx.is_asset_selected(&full_path) {
                            // Multi-selection: batch delete
                            let label = format!("Delete {} assets", sel_count);
                            if ui.button(&label).clicked() {
                                // Delete all selected assets
                                for p in &self.editor_ctx.selected_assets.clone() {
                                    if p.is_dir() {
                                        let _ = std::fs::remove_dir_all(p);
                                    } else {
                                        let _ = std::fs::remove_file(p);
                                    }
                                }
                                self.editor_ctx.selected_assets.clear();
                                self.editor_ctx.refresh_assets();
                                ui.close();
                            }
                        } else {
                            if ui.button("Delete").clicked() {
                                self.editor_ctx.pending_delete_asset = Some(full_path.clone());
                                ui.close();
                            }
                        }
                    });

                    // Drag-and-drop: set payload for mesh, material, and clip files
                    if is_draggable_asset {
                        // If multi-selected and this item is in selection, drag all selected
                        if self.editor_ctx.selected_assets.len() > 1 && self.editor_ctx.is_asset_selected(&full_path) {
                            let paths: Vec<String> = self.editor_ctx.selected_assets.iter()
                                .map(|p| p.to_string_lossy().into_owned())
                                .collect();
                            resp.dnd_set_drag_payload(paths);
                        } else {
                            resp.dnd_set_drag_payload(vec![full_path.to_string_lossy().into_owned()]);
                        }
                    }

                    // Non-DnD items/folders: drag starts lasso selection
                    if !is_draggable_asset && resp.drag_started() && self.editor_ctx.lasso_origin.is_none() {
                        if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                            let shift = ui.input(|i| i.modifiers.shift);
                            self.editor_ctx.pre_lasso_selection = if shift {
                                self.editor_ctx.selected_assets.clone()
                            } else {
                                Vec::new()
                            };
                            self.editor_ctx.lasso_origin = Some(pos);
                        }
                    }

                    if is_folder {
                        if resp.double_clicked() {
                            self.editor_ctx.asset_current_dir = full_path;
                            self.editor_ctx.refresh_assets();
                        }
                    } else {
                        // Click handling with multi-select modifiers
                        if resp.clicked() {
                            let shift = ui.input(|i| i.modifiers.shift);
                            let cmd = ui.input(|i| i.modifiers.command);

                            if shift {
                                // Shift+click: range select
                                let current_dir = self.editor_ctx.asset_current_dir.clone();
                                self.editor_ctx.select_asset_range(&entries, entry_index, &current_dir);
                            } else if cmd {
                                // Cmd/Ctrl+click: toggle individual
                                self.editor_ctx.toggle_asset(full_path.clone());
                            } else {
                                // Plain click: replace selection
                                self.editor_ctx.select_asset(full_path.clone());
                            }
                            self.editor_ctx.last_selected_asset_index = Some(entry_index);
                            // Reset clip preview when selection changes
                            self.editor_ctx.clip_preview_playing = false;
                            self.editor_ctx.clip_preview_time = 0.0;
                        }
                        // Double click: load mesh into scene
                        if resp.double_clicked() && is_mesh {
                            self.editor_ctx.pending_load_asset = Some(
                                full_path.to_string_lossy().into_owned(),
                            );
                        }
                    }
                }
            });

            // Remaining space for right-click on empty area + lasso drag
            let remaining = ui.available_size();
            let (_rect, bg_response) = ui.allocate_exact_size(
                egui::vec2(remaining.x.max(1.0), remaining.y.max(30.0)),
                egui::Sense::click_and_drag(),
            );
            // Drag on empty space below grid starts lasso
            if bg_response.drag_started() && self.editor_ctx.lasso_origin.is_none() {
                if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                    let shift = ui.input(|i| i.modifiers.shift);
                    self.editor_ctx.pre_lasso_selection = if shift {
                        self.editor_ctx.selected_assets.clone()
                    } else {
                        Vec::new()
                    };
                    self.editor_ctx.lasso_origin = Some(pos);
                }
            }
            // Click on empty space clears selection
            if bg_response.clicked() {
                self.editor_ctx.selected_assets.clear();
            }
            bg_response.context_menu(|ui| {
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

            // ---- Lasso rectangle selection ----
            if let Some(origin) = self.editor_ctx.lasso_origin {
                let primary_down = ui.input(|i| i.pointer.primary_down());
                if primary_down {
                    if let Some(current) = ui.input(|i| i.pointer.hover_pos()) {
                        let lasso_rect = egui::Rect::from_two_pos(origin, current);

                        // Draw lasso rectangle overlay
                        let painter = ui.painter();
                        use crate::editor::theme;
                        let lasso_fill = Color32::from_rgba_unmultiplied(
                            theme::ACCENT.r(), theme::ACCENT.g(), theme::ACCENT.b(), 30,
                        );
                        let lasso_stroke = Color32::from_rgba_unmultiplied(
                            theme::ACCENT.r(), theme::ACCENT.g(), theme::ACCENT.b(), 180,
                        );
                        painter.rect_filled(lasso_rect, 0.0, lasso_fill);
                        painter.rect_stroke(
                            lasso_rect, 0.0,
                            egui::Stroke::new(1.0, lasso_stroke),
                            egui::StrokeKind::Outside,
                        );

                        // Compute selection from intersection
                        let mut new_sel = self.editor_ctx.pre_lasso_selection.clone();
                        for (path, item_rect) in &item_rects {
                            if lasso_rect.intersects(*item_rect) && !new_sel.iter().any(|p| p == path) {
                                new_sel.push(path.clone());
                            }
                        }
                        self.editor_ctx.selected_assets = new_sel;

                        ui.ctx().request_repaint();
                    }
                } else {
                    // Primary released -> end lasso
                    self.editor_ctx.lasso_origin = None;
                    self.editor_ctx.pre_lasso_selection.clear();
                }
            }
        });

        // Track current scroll offset every frame
        self.editor_ctx.lasso_scroll_offset = scroll_out.state.offset.y;

        // Auto-scroll during lasso when pointer is outside visible area
        if self.editor_ctx.lasso_origin.is_some() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                let visible = scroll_out.inner_rect;
                let speed = 8.0;
                if pos.y < visible.top() {
                    let d = (visible.top() - pos.y).clamp(1.0, 80.0) / 80.0 * speed;
                    self.editor_ctx.lasso_scroll_offset =
                        (self.editor_ctx.lasso_scroll_offset - d).max(0.0);
                    ui.ctx().request_repaint();
                } else if pos.y > visible.bottom() {
                    let d = (pos.y - visible.bottom()).clamp(1.0, 80.0) / 80.0 * speed;
                    self.editor_ctx.lasso_scroll_offset += d;
                    ui.ctx().request_repaint();
                }
            }
        }
    }
}
