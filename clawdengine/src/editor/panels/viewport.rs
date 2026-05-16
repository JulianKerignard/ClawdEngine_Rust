use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_viewport(&mut self, ui: &mut egui::Ui) {
        ui.painter()
            .rect_filled(ui.available_rect_before_wrap(), 0.0, theme::BG_CRUST);

        // ---- Viewport toolbar ----
        self.draw_viewport_toolbar(ui);

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
                    theme::MAUVE
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
                                    egui::Stroke::new(1.0, theme::MAUVE),
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
                                theme::ACCENT
                            } else {
                                Color32::from_white_alpha(40)
                            };
                            painter.rect_stroke(
                                rect, 4.0,
                                egui::Stroke::new(if child_selected { 2.0 } else { 1.0 }, stroke_color),
                                egui::StrokeKind::Outside,
                            );
                            // Selection handles for child panels
                            if child_selected {
                                draw_selection_handles(painter, rect);
                            }
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
                                egui::Stroke::new(1.0, theme::ACCENT),
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
                                egui::Stroke::new(2.0, theme::ACCENT),
                                egui::StrokeKind::Outside,
                            );
                            draw_selection_handles(painter, rect);
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

        // ---- Stats overlay (styled card, top-left) ----
        if self.editor_ctx.show_stats_overlay {
            let vp_rect = self.editor_ctx.viewport_rect;
            let overlay_pos = egui::pos2(vp_rect.left() + 8.0, vp_rect.top() + 8.0);
            egui::Area::new(egui::Id::new("stats_overlay"))
                .fixed_pos(overlay_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Frame::NONE
                        .fill(Color32::from_black_alpha(180))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.style_mut().spacing.item_spacing.y = 2.0;
                            let mono = egui::FontId::new(10.5, egui::FontFamily::Monospace);
                            let ec = &self.editor_ctx;
                            // Right-padded values so the overlay width is
                            // stable across frames (an Area's frame grows to
                            // fit its content, so jittering numeric widths
                            // would pulse the panel size).
                            let frame_ms = if ec.fps > 0.0 {
                                format!("{:>5.1}ms", 1000.0 / ec.fps)
                            } else {
                                "    —".to_string()
                            };
                            let rows: &[(&str, String)] = &[
                                ("FPS",   format!("{:>5.0}", ec.fps)),
                                ("Frame", frame_ms),
                                ("Draws", format!("{:>5}", ec.draw_calls)),
                                ("Tris",  format!("{:>7}", ec.visible_triangles)),
                                ("VRAM",  "    —".to_string()),
                            ];
                            for (key, val) in rows {
                                ui.horizontal(|ui| {
                                    ui.style_mut().spacing.item_spacing.x = 8.0;
                                    ui.label(
                                        egui::RichText::new(*key)
                                            .font(mono.clone())
                                            .color(theme::TEXT_DISABLED),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(val.as_str())
                                                    .font(mono.clone())
                                                    .color(theme::TEXT_PRIMARY),
                                            );
                                        },
                                    );
                                });
                            }
                        });
                });
        }

        // ---- Scene gizmo (orientation cube, top-right) ----
        {
            let vp_rect = self.editor_ctx.viewport_rect;
            let gizmo_size = 72.0_f32;
            let gizmo_pos = egui::pos2(
                vp_rect.right() - gizmo_size - 8.0,
                vp_rect.top() + 8.0,
            );
            egui::Area::new(egui::Id::new("scene_gizmo_orient"))
                .fixed_pos(gizmo_pos)
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(gizmo_size, gizmo_size),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);
                    draw_orientation_gizmo(&painter, rect);
                });
        }

        // ---- Viewport footer (bottom bar) ----
        {
            let vp_rect = self.editor_ctx.viewport_rect;
            let footer_h = 22.0_f32;
            let footer_pos = egui::pos2(vp_rect.left(), vp_rect.bottom() - footer_h);
            egui::Area::new(egui::Id::new("viewport_footer"))
                .fixed_pos(footer_pos)
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    let footer_rect = egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(vp_rect.width(), footer_h),
                    );
                    // Gradient bg: black-alpha at bottom fading to transparent
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(vp_rect.width(), footer_h),
                        ),
                        0.0,
                        Color32::from_black_alpha(100),
                    );

                    let mono_sm = egui::FontId::new(10.0, egui::FontFamily::Monospace);
                    let painter = ui.painter_at(footer_rect);

                    // Left: cam info (mock values — no camera transform accessible here)
                    let cam_text = "Cam (0.0, 1.0, -5.0) · Yaw 0° · Pitch 0°";
                    painter.text(
                        egui::pos2(10.0, footer_h * 0.5),
                        egui::Align2::LEFT_CENTER,
                        cam_text,
                        mono_sm.clone(),
                        theme::TEXT_DISABLED,
                    );

                    // Right: hot-reload badge in ACCENT
                    let hot_text = "● hot-reload · 14 files";
                    painter.text(
                        egui::pos2(vp_rect.width() - 10.0, footer_h * 0.5),
                        egui::Align2::RIGHT_CENTER,
                        hot_text,
                        mono_sm,
                        theme::ACCENT,
                    );
                });
        }
    }

    /// Thin toolbar drawn above the viewport texture (inside the panel area).
    fn draw_viewport_toolbar(&mut self, ui: &mut egui::Ui) {
        let toolbar_h = 28.0_f32;

        Frame::NONE
            .fill(theme::BG_MANTLE)
            .inner_margin(Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.set_min_height(toolbar_h);
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 4.0;

                    // ---- Left group: shading / view toggles ----
                    vt_button(ui, "Shaded \u{25BE}", true);
                    vt_separator(ui);

                    // Grid toggle reflects real state
                    let grid_active = self.editor_ctx.show_grid;
                    if vt_button(ui, "Grid", grid_active).clicked() {
                        self.editor_ctx.show_grid = !self.editor_ctx.show_grid;
                    }

                    // Gizmos toggle reflects stats overlay (closest proxy)
                    let gizmos_active = self.editor_ctx.show_stats_overlay;
                    if vt_button(ui, "Gizmos", gizmos_active).clicked() {
                        self.editor_ctx.show_stats_overlay = !self.editor_ctx.show_stats_overlay;
                    }

                    vt_button(ui, "Snap", false);

                    // ---- Spacer ----
                    ui.add_space(ui.available_width() - 90.0);

                    // ---- Right: perspective label ----
                    ui.label(
                        egui::RichText::new("Persp \u{00B7} FOV 60\u{00B0}")
                            .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                            .color(theme::TEXT_DISABLED),
                    );
                });
            });

        // Bottom border separator
        let toolbar_rect = ui.min_rect();
        ui.painter().hline(
            toolbar_rect.x_range(),
            toolbar_rect.bottom(),
            Stroke::new(1.0, theme::BG_CRUST),
        );
    }
}

// ---- Helpers ----

/// Small toolbar button. Returns the response so callers can check .clicked().
fn vt_button(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let fill = if active { theme::BG_SURFACE1 } else { Color32::TRANSPARENT };
    let text_color = if active { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };

    let btn = egui::Button::new(
        egui::RichText::new(label)
            .font(egui::FontId::new(11.0, egui::FontFamily::Proportional))
            .color(text_color),
    )
    .fill(fill)
    .stroke(Stroke::NONE)
    .corner_radius(CornerRadius::same(3))
    .min_size(egui::vec2(0.0, 22.0));

    ui.add(btn)
}

/// Thin vertical divider for the toolbar.
fn vt_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 16.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme::BG_CRUST);
}

/// Draws 4 corner handles (filled squares) in ACCENT around `rect`.
fn draw_selection_handles(painter: &egui::Painter, rect: egui::Rect) {
    let handle_sz = 5.0_f32;
    let half = handle_sz * 0.5;
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ];
    for corner in &corners {
        let handle_rect = egui::Rect::from_center_size(*corner, egui::vec2(handle_sz, handle_sz));
        painter.rect_filled(handle_rect, 1.0, theme::ACCENT);
        painter.rect_stroke(
            handle_rect,
            1.0,
            Stroke::new(1.0, theme::ON_ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    let _ = half; // suppress unused warning
}

/// Draws an isometric-style orientation gizmo (3-face cube) using the painter.
fn draw_orientation_gizmo(painter: &egui::Painter, rect: egui::Rect) {
    let cx = rect.center().x;
    let cy = rect.center().y;

    // Face vertices for a simplified isometric cube (top, right, left faces)
    // Top face (Y axis — green)
    let top_face = [
        egui::pos2(cx,        cy - 24.0),
        egui::pos2(cx + 18.0, cy - 14.0),
        egui::pos2(cx,        cy - 4.0),
        egui::pos2(cx - 18.0, cy - 14.0),
    ];
    // Right face (X axis — red)
    let right_face = [
        egui::pos2(cx,        cy - 4.0),
        egui::pos2(cx + 18.0, cy - 14.0),
        egui::pos2(cx + 18.0, cy + 14.0),
        egui::pos2(cx,        cy + 24.0),
    ];
    // Left face (Z axis — blue)
    let left_face = [
        egui::pos2(cx,        cy - 4.0),
        egui::pos2(cx - 18.0, cy - 14.0),
        egui::pos2(cx - 18.0, cy + 14.0),
        egui::pos2(cx,        cy + 24.0),
    ];

    let fill_y = Color32::from_rgba_unmultiplied(0x7E, 0xD9, 0x57, 50);
    let fill_x = Color32::from_rgba_unmultiplied(0xFF, 0x6B, 0x6B, 50);
    let fill_z = Color32::from_rgba_unmultiplied(0x5E, 0xB1, 0xFF, 50);

    // Filled faces
    painter.add(egui::Shape::convex_polygon(top_face.to_vec(),   fill_y, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(right_face.to_vec(), fill_x, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(left_face.to_vec(),  fill_z, Stroke::NONE));

    // Stroked outlines
    for (pts, col) in [
        (&top_face[..],   theme::AXIS_Y),
        (&right_face[..], theme::AXIS_X),
        (&left_face[..],  theme::AXIS_Z),
    ] {
        let path: Vec<egui::Pos2> = pts.to_vec();
        painter.add(egui::Shape::closed_line(path, Stroke::new(1.0, col)));
    }

    // Axis labels
    let mono = egui::FontId::new(9.0, egui::FontFamily::Monospace);
    painter.text(egui::pos2(cx + 14.0, cy + 2.0),  egui::Align2::LEFT_CENTER,   "X", mono.clone(), theme::AXIS_X);
    painter.text(egui::pos2(cx - 3.0,  cy - 12.0), egui::Align2::CENTER_BOTTOM, "Y", mono.clone(), theme::AXIS_Y);
    painter.text(egui::pos2(cx - 18.0, cy + 2.0),  egui::Align2::RIGHT_CENTER,  "Z", mono.clone(), theme::AXIS_Z);
}
