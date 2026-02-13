use egui::{Color32, CornerRadius, Frame, Margin};

use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_viewport(&mut self, ui: &mut egui::Ui) {
        ui.painter()
            .rect_filled(ui.available_rect_before_wrap(), 0.0, theme::BG_CRUST);

        if let Some(tex_id) = self.viewport_texture {
            let available = ui.available_size();
            ui.image(egui::load::SizedTexture::new(tex_id, available));
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Viewport");
            });
        }
        self.editor_ctx.viewport_rect = ui.min_rect();

        // Drop zone: accept dragged assets from the asset browser
        let drop_resp = ui.interact(
            self.editor_ctx.viewport_rect,
            ui.id().with("viewport_drop"),
            egui::Sense::hover(),
        );
        if let Some(payload) = drop_resp.dnd_release_payload::<String>() {
            self.editor_ctx.pending_load_asset = Some((*payload).clone());
            // Store cursor position relative to viewport for spawn placement
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                let vp = self.editor_ctx.viewport_rect;
                let sx = pointer.x - vp.left();
                let sy = pointer.y - vp.top();
                self.editor_ctx.pending_drop_screen_pos = Some((sx, sy));
            }
        }

        // ---- Canvas + UI Element overlay (visible in editor) ----
        {
            let vp_rect = self.editor_ctx.viewport_rect;
            let painter = ui.painter();

            // Draw Canvas boundaries
            for eid in self.world.iter_entities() {
                let Some(cv) = self.world.get_canvas(eid) else { continue };
                if !cv.visible { continue; }

                // Scale Canvas to fit viewport while preserving aspect ratio
                let vp_w = vp_rect.width();
                let vp_h = vp_rect.height();
                let scale = (vp_w / cv.width).min(vp_h / cv.height).min(1.0);
                let canvas_w = cv.width * scale;
                let canvas_h = cv.height * scale;
                let canvas_x = vp_rect.left() + (vp_w - canvas_w) * 0.5;
                let canvas_y = vp_rect.top() + (vp_h - canvas_h) * 0.5;
                let canvas_rect = egui::Rect::from_min_size(
                    egui::pos2(canvas_x, canvas_y),
                    egui::vec2(canvas_w, canvas_h),
                );

                let is_selected = self.editor_ctx.is_selected(eid);
                let border_color = if is_selected {
                    Color32::from_rgb(0xCB, 0xA6, 0xF7)
                } else {
                    Color32::from_rgba_premultiplied(0xCB, 0xA6, 0xF7, 60)
                };
                painter.rect_stroke(
                    canvas_rect, 0.0,
                    egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color),
                    egui::StrokeKind::Outside,
                );

                // Canvas label
                let name = self.world.get_name(eid).unwrap_or("Canvas");
                painter.text(
                    egui::pos2(canvas_rect.left() + 4.0, canvas_rect.top() - 16.0),
                    egui::Align2::LEFT_BOTTOM,
                    name,
                    egui::FontId::proportional(11.0),
                    border_color,
                );

                // Render child UI elements relative to this canvas rect
                for &child_eid in self.world.get_children(eid) {
                    let Some(el) = self.world.get_ui_element(child_eid) else { continue };
                    if !el.visible { continue; }

                    let anchor_pos = crate::editor::layout::resolve_anchor(el.anchor, canvas_rect);
                    let pos = anchor_pos + egui::vec2(el.offset[0] * scale, el.offset[1] * scale);
                    let r = (el.color.x * 255.0) as u8;
                    let g = (el.color.y * 255.0) as u8;
                    let b = (el.color.z * 255.0) as u8;
                    let a = (el.alpha * 255.0) as u8;
                    let color = Color32::from_rgba_unmultiplied(r, g, b, a);
                    let child_selected = self.editor_ctx.is_selected(child_eid);

                    match el.kind {
                        crate::core::UiElementKind::Text => {
                            let galley = painter.layout_no_wrap(
                                el.text.clone(),
                                egui::FontId::proportional(el.font_size * scale),
                                color,
                            );
                            let text_rect = egui::Rect::from_min_size(pos, galley.size());
                            painter.galley(pos, galley, Color32::TRANSPARENT);
                            if child_selected {
                                painter.rect_stroke(
                                    text_rect.expand(2.0), 2.0,
                                    egui::Stroke::new(1.0, Color32::from_rgb(0xCB, 0xA6, 0xF7)),
                                    egui::StrokeKind::Outside,
                                );
                            }
                        }
                        crate::core::UiElementKind::Panel => {
                            let rect = egui::Rect::from_min_size(
                                pos,
                                egui::vec2(el.size[0] * scale, el.size[1] * scale),
                            );
                            painter.rect_filled(rect, 4.0, color);
                            let stroke_color = if child_selected {
                                Color32::from_rgb(0xCB, 0xA6, 0xF7)
                            } else {
                                Color32::from_white_alpha(40)
                            };
                            painter.rect_stroke(
                                rect, 4.0,
                                egui::Stroke::new(if child_selected { 2.0 } else { 1.0 }, stroke_color),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }

            // Render orphan UI elements (not under any Canvas) relative to viewport
            for eid in self.world.iter_entities() {
                let Some(el) = self.world.get_ui_element(eid) else { continue };
                if !el.visible { continue; }
                // Skip if parented under a Canvas
                if let Some(parent) = self.world.get_parent(eid) {
                    if self.world.get_canvas(parent).is_some() { continue; }
                }

                let anchor_pos = crate::editor::layout::resolve_anchor(el.anchor, vp_rect);
                let pos = anchor_pos + egui::vec2(el.offset[0], el.offset[1]);
                let r = (el.color.x * 255.0) as u8;
                let g = (el.color.y * 255.0) as u8;
                let b = (el.color.z * 255.0) as u8;
                let a = (el.alpha * 255.0) as u8;
                let color = Color32::from_rgba_unmultiplied(r, g, b, a);
                let is_selected = self.editor_ctx.is_selected(eid);

                match el.kind {
                    crate::core::UiElementKind::Text => {
                        let galley = painter.layout_no_wrap(
                            el.text.clone(),
                            egui::FontId::proportional(el.font_size),
                            color,
                        );
                        let text_rect = egui::Rect::from_min_size(pos, galley.size());
                        painter.galley(pos, galley, Color32::TRANSPARENT);
                        if is_selected {
                            painter.rect_stroke(
                                text_rect.expand(2.0), 2.0,
                                egui::Stroke::new(1.0, Color32::from_rgb(0xCB, 0xA6, 0xF7)),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                    crate::core::UiElementKind::Panel => {
                        let rect = egui::Rect::from_min_size(pos, egui::vec2(el.size[0], el.size[1]));
                        painter.rect_filled(rect, 4.0, color);
                        if is_selected {
                            painter.rect_stroke(
                                rect, 4.0,
                                egui::Stroke::new(2.0, Color32::from_rgb(0xCB, 0xA6, 0xF7)),
                                egui::StrokeKind::Outside,
                            );
                        } else {
                            painter.rect_stroke(
                                rect, 4.0,
                                egui::Stroke::new(1.0, Color32::from_white_alpha(40)),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
        }

        if self.editor_ctx.show_stats_overlay {
            let vp_rect = self.editor_ctx.viewport_rect;
            let overlay_pos = egui::pos2(vp_rect.left() + 8.0, vp_rect.top() + 8.0);
            egui::Area::new(egui::Id::new("stats_overlay"))
                .fixed_pos(overlay_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Frame::NONE
                        .fill(egui::Color32::from_black_alpha(200))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.style_mut().spacing.item_spacing.y = 3.0;
                            let ec = &self.editor_ctx;
                            ui.label(
                                egui::RichText::new(format!("{:.0} FPS", ec.fps))
                                    .color(Color32::from_rgb(0xA6, 0xE3, 0xA1))
                                    .monospace()
                                    .strong()
                                    .size(14.0),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} entities  \u{2022}  {} draws  \u{2022}  {} tris", ec.entity_count, ec.draw_calls, ec.visible_triangles))
                                    .color(Color32::from_rgb(0xA6, 0xAD, 0xC8))
                                    .monospace()
                                    .size(11.0),
                            );
                        });
                });
        }
    }
}
