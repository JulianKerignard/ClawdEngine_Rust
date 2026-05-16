use egui::Color32;

use crate::editor::context::{AssetModal, SpawnRequest};
use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;
use crate::core::EntityId;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_hierarchy(&mut self, ui: &mut egui::Ui) {
        // Cancel drag on Escape
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor_ctx.hierarchy_drag_source = None;
        }

        // Ghost label following cursor during drag
        if let Some(src) = self.editor_ctx.hierarchy_drag_source {
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                let drag_name = self.world.get_name(src).unwrap_or("Entity");
                let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("hierarchy_drag"));
                ui.ctx().layer_painter(layer).text(
                    pointer_pos + egui::vec2(12.0, -8.0),
                    egui::Align2::LEFT_CENTER,
                    drag_name,
                    egui::FontId::proportional(12.0),
                    theme::ACCENT,
                );
            }
        }

        // ---- Scene header (BG_SURFACE0 band with accent dot + scene name + entity count) ----
        {
            let scene_name = self.editor_ctx.scene_name.clone();
            let entity_count = self.world.entity_count();

            let header_height = 24.0;
            let (header_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), header_height),
                egui::Sense::hover(),
            );

            // Background
            ui.painter().rect_filled(header_rect, 0.0, theme::BG_SURFACE0);

            let center_y = header_rect.center().y;
            let mut x = header_rect.left() + 8.0;

            // Accent dot (~6px radius)
            let dot_center = egui::pos2(x + 3.0, center_y);
            ui.painter().circle_filled(dot_center, 3.0, theme::ACCENT);
            x += 12.0;

            // Scene name
            let name_galley = ui.painter().layout_no_wrap(
                scene_name.clone(),
                egui::FontId::proportional(11.0),
                theme::TEXT_PRIMARY,
            );
            ui.painter().galley(egui::pos2(x, center_y - name_galley.size().y / 2.0), name_galley, theme::TEXT_PRIMARY);

            // Entity count (right-aligned, mono, disabled)
            let count_text = format!("{entity_count} entities");
            let count_galley = ui.painter().layout_no_wrap(
                count_text,
                egui::FontId::monospace(10.0),
                theme::TEXT_DISABLED,
            );
            let count_x = header_rect.right() - 8.0 - count_galley.size().x;
            ui.painter().galley(
                egui::pos2(count_x, center_y - count_galley.size().y / 2.0),
                count_galley,
                theme::TEXT_DISABLED,
            );

            // Bottom separator
            ui.painter().hline(
                header_rect.x_range(),
                header_rect.bottom(),
                egui::Stroke::new(1.0, theme::BG_CRUST),
            );
        }

        // ---- Toolbar: Add + Search ----
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let add_btn = ui.button("+");
                if add_btn.on_hover_text("Add entity").clicked() {
                    self.editor_ctx.show_add_menu = !self.editor_ctx.show_add_menu;
                }
            });
        });

        // Search bar
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("\u{1F50D}").size(12.0).color(theme::TEXT_DISABLED));
            ui.add(
                egui::TextEdit::singleline(&mut self.editor_ctx.hierarchy_search)
                    .hint_text("Search entities...")
                    .desired_width(ui.available_width()),
            );
        });

        if self.editor_ctx.show_add_menu {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.menu_button("3D Object", |ui| {
                    if ui.button("Empty").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Empty);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                    if ui.button("Cube").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Cube);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                    if ui.button("Sphere").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Sphere);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                });
                if ui.button("Light").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Light);
                    self.editor_ctx.show_add_menu = false;
                }
                if ui.button("Camera").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Camera);
                    self.editor_ctx.show_add_menu = false;
                }
                if ui.button("Audio").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Audio);
                    self.editor_ctx.show_add_menu = false;
                }
                ui.menu_button("UI", |ui| {
                    if ui.button("Canvas").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Canvas);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                    if ui.button("Text").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::UiText);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                    if ui.button("Panel").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::UiPanel);
                        self.editor_ctx.show_add_menu = false;
                        ui.close();
                    }
                });
            });
        }
        ui.separator();

        let search_query = self.editor_ctx.hierarchy_search.to_lowercase();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            // Only show root entities (without parent)
            let roots: Vec<EntityId> = self.world.iter_entities()
                .filter(|&eid| self.world.get_parent(eid).is_none())
                .collect();

            for entity_id in roots {
                // Filter by search query
                if !search_query.is_empty() {
                    let name = self.world.get_name(entity_id).unwrap_or("Entity");
                    let has_match = name.to_lowercase().contains(&search_query)
                        || self.subtree_matches_search(entity_id, &search_query);
                    if !has_match { continue; }
                }
                self.show_entity_node(ui, entity_id, 0);
            }

            let remaining = ui.available_size();
            let (_, empty_resp) = ui.allocate_exact_size(
                egui::vec2(remaining.x.max(1.0), remaining.y.max(60.0)),
                egui::Sense::click(),
            );

            // Drop on empty zone = detach from parent (become root)
            if let Some(src) = self.editor_ctx.hierarchy_drag_source {
                if empty_resp.hovered() && ui.input(|i| i.pointer.any_released()) {
                    self.editor_ctx.pending_reparent = Some((src, None));
                    self.editor_ctx.hierarchy_drag_source = None;
                }
            }

            empty_resp.context_menu(|ui| {
                ui.menu_button("3D Object", |ui| {
                    if ui.button("Empty").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Empty);
                        ui.close();
                    }
                    if ui.button("Cube").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Cube);
                        ui.close();
                    }
                    if ui.button("Sphere").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Sphere);
                        ui.close();
                    }
                });
                if ui.button("Light").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Light);
                    ui.close();
                }
                if ui.button("Camera").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Camera);
                    ui.close();
                }
                if ui.button("Audio").clicked() {
                    self.editor_ctx.pending_spawn = Some(SpawnRequest::Audio);
                    ui.close();
                }
                ui.menu_button("UI", |ui| {
                    if ui.button("Canvas").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::Canvas);
                        ui.close();
                    }
                    if ui.button("Text").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::UiText);
                        ui.close();
                    }
                    if ui.button("Panel").clicked() {
                        self.editor_ctx.pending_spawn = Some(SpawnRequest::UiPanel);
                        ui.close();
                    }
                });
                ui.separator();
                if ui.button("New Scene").clicked() {
                    self.editor_ctx.asset_modal = Some(AssetModal::NewScene { name: "New Scene".to_string() });
                    ui.close();
                }
            });
        });

        if !self.editor_ctx.selected_entities.is_empty() {
            ui.separator();
            if ui.button("Delete").clicked() {
                self.editor_ctx.pending_delete = self.editor_ctx.selected_entities.clone();
            }
        }
    }

    fn subtree_matches_search(&self, eid: EntityId, query: &str) -> bool {
        for child in self.world.get_children(eid).to_vec() {
            let name = self.world.get_name(child).unwrap_or("Entity");
            if name.to_lowercase().contains(query) {
                return true;
            }
            if self.subtree_matches_search(child, query) {
                return true;
            }
        }
        false
    }

    /// Determine icon glyph and tint color for an entity.
    ///
    /// Tint discipline (anti-orange overuse):
    ///   - Camera  → SKY
    ///   - Light   → WARNING (warm yellow)
    ///   - Audio   → SUCCESS (green)
    ///   - Canvas / UI / Rect → MAUVE (purple)
    ///   - Player name or entity has a script attached → ACCENT (orange)
    ///   - Mesh / group / default → TEXT_SECONDARY (neutral gray)
    fn hierarchy_icon(&self, entity_id: EntityId) -> (&'static str, Color32) {
        let name = self.world.get_name(entity_id).unwrap_or("");
        let name_lower = name.to_ascii_lowercase();

        // Check for script attachment (scripts vec is (EntityId, Box<dyn GameScript>))
        let has_script = self.scripts.iter().any(|(sid, _)| *sid == entity_id);

        if self.world.get_canvas(entity_id).is_some() {
            // Canvas / UI container
            ("\u{1F5BC}", theme::MAUVE)
        } else if self.world.get_camera(entity_id).is_some() {
            ("\u{1F3A5}", theme::SKY)
        } else if self.world.get_light(entity_id).is_some() {
            ("\u{2600}", theme::WARNING)
        } else if self.world.get_audio_source(entity_id).is_some() {
            ("\u{1F50A}", theme::SUCCESS)
        } else if self.world.get_ui_element(entity_id).is_some() {
            ("\u{1F5B5}", theme::MAUVE)
        } else if has_script || name_lower.contains("player") {
            // Gameplay / scripted entities get the orange accent
            ("\u{25A0}", theme::ACCENT)
        } else if self.world.get_mesh_renderer(entity_id).is_some() {
            // Plain mesh — neutral, not orange
            ("\u{25A0}", theme::TEXT_SECONDARY)
        } else {
            // Empty group / transform
            ("\u{25CB}", theme::TEXT_SECONDARY)
        }
    }

    fn show_entity_node(&mut self, ui: &mut egui::Ui, entity_id: EntityId, depth: u32) {
        let name = self.world.get_name(entity_id).unwrap_or("Entity");
        let name_owned = name.to_string();
        let name_lower = name_owned.to_ascii_lowercase();

        let is_selected = self.editor_ctx.is_selected(entity_id);
        let has_children = !self.world.get_children(entity_id).is_empty();
        let is_expanded = self.editor_ctx.hierarchy_expanded.contains(&entity_id);

        // Spec: height 22px, indent depth*12 + 4px left padding
        let row_height = 22.0;
        let indent = depth as f32 * 12.0 + 4.0;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_height),
            egui::Sense::click_and_drag(),
        );

        // ---- Drag start ----
        if response.drag_started() {
            self.editor_ctx.hierarchy_drag_source = Some(entity_id);
        }

        // ---- Drop target detection + visual indicator ----
        let drag_source = self.editor_ctx.hierarchy_drag_source;
        if let Some(src) = drag_source {
            if response.hovered() && src != entity_id && !self.world.is_ancestor(entity_id, src) {
                let drop_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + indent + 16.0, rect.bottom() - 2.0),
                    egui::vec2(rect.width() - indent - 16.0, 2.0),
                );
                ui.painter().rect_filled(drop_rect, 1.0, theme::ACCENT);

                if ui.input(|i| i.pointer.any_released()) {
                    self.editor_ctx.pending_reparent = Some((src, Some(entity_id)));
                    self.editor_ctx.hierarchy_drag_source = None;
                    self.editor_ctx.hierarchy_expanded.insert(entity_id);
                }
            }
        }

        // ---- Row background ----
        if is_selected {
            // ACCENT_SOFT fill
            ui.painter().rect_filled(rect, 0.0, theme::ACCENT_SOFT);
            // 2px left accent bar
            let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
            ui.painter().rect_filled(bar, 0.0, theme::ACCENT);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, theme::BG_SURFACE1);
        }

        let center_y = rect.center().y;

        // ---- Twirl (chevron) — always reserve 12px, only draw if has_children ----
        // Arrow x-position: from left edge + indent
        let twirl_x = rect.left() + indent;
        let twirl_area_w = 12.0;
        if has_children {
            let arrow = if is_expanded { "\u{25BC}" } else { "\u{25B6}" }; // ▼ / ▶
            ui.painter().text(
                egui::pos2(twirl_x + twirl_area_w / 2.0, center_y),
                egui::Align2::CENTER_CENTER,
                arrow,
                egui::FontId::proportional(9.0),
                theme::TEXT_DISABLED,
            );
        }
        // (if no children: space is reserved but nothing drawn — "visibility:hidden" equiv)

        // ---- Icon (14×14 area) ----
        let icon_x = twirl_x + twirl_area_w + 2.0;
        let icon_area_w = 14.0;
        let (icon, base_icon_color) = self.hierarchy_icon(entity_id);

        // Disabled entities get attenuated color
        let is_disabled = self.world.get_mesh_renderer(entity_id)
            .map(|mr| !mr.visible)
            .unwrap_or(false);
        let icon_color = if is_disabled {
            base_icon_color.gamma_multiply(0.4)
        } else {
            base_icon_color
        };

        ui.painter().text(
            egui::pos2(icon_x + icon_area_w / 2.0, center_y),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(13.0),
            icon_color,
        );

        // ---- Name (flex, ellipsed via max_width) ----
        let name_x = icon_x + icon_area_w + 4.0;

        let base_name_color = if is_selected {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        let name_color = if is_disabled {
            base_name_color.gamma_multiply(0.4)
        } else {
            base_name_color
        };

        // Reserve right side for: eye (12px + 4px pad) + optional tag
        // Tag pill width estimate: ~40px for "Player"
        let has_player_tag = name_lower.contains("player");
        let right_reserve = if response.hovered() { 20.0 } else { 4.0 }
            + if has_player_tag { 44.0 } else { 0.0 };
        let name_max_width = (rect.right() - name_x - right_reserve).max(20.0);

        ui.painter().text(
            egui::pos2(name_x, center_y),
            egui::Align2::LEFT_CENTER,
            truncate_name(&name_owned, name_max_width, ui),
            egui::FontId::proportional(11.5),
            name_color,
        );

        // ---- "Player" tag pill ----
        // Position tag after name, or near right (right-aligned before eye)
        if has_player_tag {
            let tag_right = rect.right() - if response.hovered() { 20.0 } else { 4.0 } - 2.0;
            let tag_text = "Player";
            let tag_font = egui::FontId::monospace(9.5);
            let tag_galley = ui.painter().layout_no_wrap(
                tag_text.to_string(),
                tag_font.clone(),
                theme::ACCENT,
            );
            let tag_w = tag_galley.size().x + 8.0; // 4px pad each side
            let tag_h = 14.0;
            let tag_rect = egui::Rect::from_min_size(
                egui::pos2(tag_right - tag_w, center_y - tag_h / 2.0),
                egui::vec2(tag_w, tag_h),
            );
            ui.painter().rect_filled(tag_rect, 2.0, theme::ACCENT_SOFT);
            ui.painter().text(
                tag_rect.center(),
                egui::Align2::CENTER_CENTER,
                tag_text,
                tag_font,
                theme::ACCENT,
            );
        }

        // ---- Visibility eye (hover-only, right edge) ----
        if response.hovered() {
            if let Some(mr) = self.world.get_mesh_renderer(entity_id) {
                let visible = mr.visible;
                let eye_icon = if visible { "\u{1F441}" } else { "\u{1F648}" };
                let eye_color = if visible {
                    theme::TEXT_DISABLED
                } else {
                    theme::ERROR.gamma_multiply(0.6)
                };
                let eye_x = rect.right() - 12.0;
                let eye_rect = egui::Rect::from_center_size(
                    egui::pos2(eye_x, center_y),
                    egui::vec2(16.0, 16.0),
                );

                // Check click on eye area
                if response.clicked() {
                    let click_x = response.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
                    if click_x > eye_x - 8.0 {
                        if let Some(mr_mut) = self.world.get_mesh_renderer_mut(entity_id) {
                            mr_mut.visible = !mr_mut.visible;
                        }
                        return;
                    }
                }

                ui.painter().text(
                    eye_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    eye_icon,
                    egui::FontId::proportional(11.0),
                    eye_color,
                );
            }
        }

        // ---- Click handling ----
        if response.clicked() {
            let click_x = response.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
            // Toggle expand/collapse when clicking the twirl zone (indent to indent+12)
            if has_children && click_x >= twirl_x && click_x < twirl_x + twirl_area_w + 4.0 {
                if is_expanded {
                    self.editor_ctx.hierarchy_expanded.remove(&entity_id);
                } else {
                    self.editor_ctx.hierarchy_expanded.insert(entity_id);
                }
            } else if ui.input(|i| i.modifiers.shift) {
                self.editor_ctx.toggle_select(entity_id);
            } else {
                self.editor_ctx.select(entity_id);
            }
        }

        // ---- Context menu ----
        response.context_menu(|ui| {
            if self.world.get_parent(entity_id).is_some() {
                if ui.button("Detach from Parent").clicked() {
                    self.editor_ctx.pending_reparent = Some((entity_id, None));
                    ui.close();
                }
            }
            if ui.button("Delete").clicked() {
                self.editor_ctx.pending_delete = vec![entity_id];
                ui.close();
            }
        });

        // ---- Recursion for children (if expanded) ----
        if has_children && is_expanded {
            let child_ids: Vec<EntityId> = self.world.get_children(entity_id).to_vec();
            for child_id in child_ids {
                self.show_entity_node(ui, child_id, depth + 1);
            }
        }
    }
}

/// Truncate a name to fit within `max_width` pixels (painter measurement not available
/// here, so we use a conservative character-based estimate then let egui clip).
/// For simplicity we just return the name as-is and rely on the fixed allocation
/// constraining the draw area — egui's text painter clips to the current clip rect.
///
/// This helper exists so the signature is explicit about the intent.
fn truncate_name<'s>(name: &'s str, _max_width: f32, _ui: &egui::Ui) -> &'s str {
    name
}
