use egui::Color32;

use crate::editor::context::{AssetModal, SpawnRequest};
use crate::editor::layout::{EditorTabViewer, entity_icon};
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

        ui.horizontal(|ui| {
            ui.strong("Hierarchy");
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

    fn show_entity_node(&mut self, ui: &mut egui::Ui, entity_id: EntityId, depth: u32) {
        let name = self.world.get_name(entity_id).unwrap_or("Entity");
        let (icon, icon_color) = entity_icon(self.world, entity_id);
        let is_selected = self.editor_ctx.is_selected(entity_id);
        let has_children = !self.world.get_children(entity_id).is_empty();
        let is_expanded = self.editor_ctx.hierarchy_expanded.contains(&entity_id);

        let indent = depth as f32 * 16.0;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 24.0),
            egui::Sense::click_and_drag(),
        );

        // Drag start
        if response.drag_started() {
            self.editor_ctx.hierarchy_drag_source = Some(entity_id);
        }

        // Drop target detection + visual indicator
        let drag_source = self.editor_ctx.hierarchy_drag_source;
        if let Some(src) = drag_source {
            if response.hovered() && src != entity_id && !self.world.is_ancestor(entity_id, src) {
                // Blue drop indicator line
                let drop_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + indent + 20.0, rect.bottom() - 2.0),
                    egui::vec2(rect.width() - indent - 20.0, 2.0),
                );
                ui.painter().rect_filled(drop_rect, 1.0, theme::ACCENT);

                // Drop execution on release
                if ui.input(|i| i.pointer.any_released()) {
                    self.editor_ctx.pending_reparent = Some((src, Some(entity_id)));
                    self.editor_ctx.hierarchy_drag_source = None;
                    self.editor_ctx.hierarchy_expanded.insert(entity_id);
                }
            }
        }

        // Clear drag state when drag stops (dropped on nothing)
        if response.drag_stopped() && self.editor_ctx.hierarchy_drag_source.is_some() {
            // Will be cleared by drop handlers above if valid, otherwise clear here
        }

        // Selection highlight (stronger + animated)
        if is_selected {
            ui.painter().rect_filled(
                rect, 4.0,
                Color32::from_rgba_premultiplied(0x89, 0xB4, 0xFA, 45),
            );
            let bar = egui::Rect::from_min_size(
                rect.left_top(), egui::vec2(3.0, rect.height()),
            );
            ui.painter().rect_filled(bar, 2.0, theme::ACCENT);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 4.0, Color32::from_white_alpha(15));
        }

        let center_y = rect.center().y;

        // Expand/collapse arrow (if has children)
        let arrow_x = rect.left() + indent + 4.0;
        if has_children {
            let arrow = if is_expanded { "▼" } else { "▶" };
            ui.painter().text(
                egui::pos2(arrow_x, center_y),
                egui::Align2::LEFT_CENTER,
                arrow,
                egui::FontId::proportional(11.0),
                theme::TEXT_DISABLED,
            );
        }

        // Icon + name (offset by indent + arrow space)
        let text_x = rect.left() + indent + 20.0;
        ui.painter().text(
            egui::pos2(text_x, center_y),
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(13.0),
            icon_color,
        );
        let name_color = if is_selected { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };
        ui.painter().text(
            egui::pos2(text_x + 20.0, center_y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(13.0),
            name_color,
        );

        // Visibility eye toggle (show on hover, right side)
        if response.hovered() {
            if let Some(mr) = self.world.get_mesh_renderer(entity_id) {
                let visible = mr.visible;
                let eye_icon = if visible { "\u{1F441}" } else { "\u{1F648}" };
                let eye_color = if visible { theme::TEXT_DISABLED } else { theme::ERROR.gamma_multiply(0.6) };
                let eye_x = rect.right() - 24.0;
                let eye_rect = egui::Rect::from_center_size(
                    egui::pos2(eye_x, center_y),
                    egui::vec2(18.0, 18.0),
                );
                // Check click on eye area
                if response.clicked() {
                    let click_x = response.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
                    if click_x > eye_x - 9.0 {
                        if let Some(mr) = self.world.get_mesh_renderer_mut(entity_id) {
                            mr.visible = !mr.visible;
                        }
                        // Don't process normal click logic below
                        return;
                    }
                }
                ui.painter().text(
                    eye_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    eye_icon,
                    egui::FontId::proportional(12.0),
                    eye_color,
                );
            }
        }

        // Click handling
        if response.clicked() {
            let click_x = response.interact_pointer_pos().map(|p| p.x).unwrap_or(0.0);
            if has_children && click_x < arrow_x + 16.0 {
                // Toggle expand/collapse
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

        // Context menu
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

        // Recursion for children (if expanded)
        if has_children && is_expanded {
            let child_ids: Vec<EntityId> = self.world.get_children(entity_id).to_vec();
            for child_id in child_ids {
                self.show_entity_node(ui, child_id, depth + 1);
            }
        }
    }
}
