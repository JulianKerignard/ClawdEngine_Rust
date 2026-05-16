use egui::{Color32, CornerRadius, Frame, Margin};

use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_game_view(&mut self, ui: &mut egui::Ui) {
        self.editor_ctx.game_view_visible = true;

        // Background (letterbox bars fill with BG_CRUST)
        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            theme::BG_CRUST,
        );

        if let Some(tex_id) = self.game_viewport_texture {
            let available = ui.available_size();

            // ---- Letterbox: enforce 16:9 aspect ratio ----
            let target_ratio = 16.0_f32 / 9.0;
            let avail_ratio = available.x / available.y.max(1.0);
            let (game_w, game_h) = if avail_ratio > target_ratio {
                // Too wide — pillarbox
                let h = available.y;
                (h * target_ratio, h)
            } else {
                // Too tall — letterbox
                let w = available.x;
                (w, w / target_ratio)
            };
            let offset_x = (available.x - game_w) * 0.5;
            let offset_y = (available.y - game_h) * 0.5;

            // Bars (already filled by the BG_CRUST background above)
            // Insert top spacer so the image is centered vertically
            if offset_y > 0.0 {
                ui.add_space(offset_y);
            }
            ui.horizontal(|ui| {
                if offset_x > 0.0 {
                    ui.add_space(offset_x);
                }
                ui.image(egui::load::SizedTexture::new(
                    tex_id,
                    egui::vec2(game_w, game_h),
                ));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No main camera in scene.\nAdd a Camera entity.");
            });
        }
        self.editor_ctx.game_viewport_rect = ui.min_rect();

        // ---- HUD overlay (play mode only) ----
        if self.editor_ctx.play_mode && self.game_viewport_texture.is_some() {
            let game_rect = self.editor_ctx.game_viewport_rect;
            let ctx = ui.ctx().clone();

            let mut scripts = std::mem::take(self.scripts);

            for (eid, script) in &mut scripts {
                let area_id = egui::Id::new("game_hud").with(eid.index);
                egui::Area::new(area_id)
                    .fixed_pos(game_rect.left_top())
                    .order(egui::Order::Foreground)
                    .show(&ctx, |ui| {
                        ui.set_clip_rect(game_rect);
                        ui.set_max_size(game_rect.size());
                        script.game_ui(ui);
                    });
            }

            *self.scripts = scripts;

            // ---- Mock HUD: HP bar + level info ----
            let hud_pos = egui::pos2(
                game_rect.left() + 16.0,
                game_rect.top() + 14.0,
            );
            egui::Area::new(egui::Id::new("game_mock_hud"))
                .fixed_pos(hud_pos)
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(game_rect);
                    Frame::NONE
                        .inner_margin(Margin::ZERO)
                        .show(ui, |ui| {
                            ui.style_mut().spacing.item_spacing.y = 3.0;
                            let mono = egui::FontId::new(12.0, egui::FontFamily::Monospace);
                            let mono_sm = egui::FontId::new(11.0, egui::FontFamily::Monospace);

                            // HP label
                            ui.label(
                                egui::RichText::new("HP  72/100")
                                    .font(mono)
                                    .color(Color32::WHITE)
                                    .strong(),
                            );

                            // HP bar — 140×6 gradient (green → yellow)
                            let (bar_rect, _) = ui.allocate_exact_size(
                                egui::vec2(140.0, 6.0),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter();
                            // Track background
                            painter.rect_filled(
                                bar_rect,
                                CornerRadius::same(3),
                                Color32::from_white_alpha(46),
                            );
                            // Fill (72% wide, gradient from SUCCESS to WARNING)
                            let fill_rect = egui::Rect::from_min_size(
                                bar_rect.min,
                                egui::vec2(bar_rect.width() * 0.72, bar_rect.height()),
                            );
                            // Simple solid fill (egui painter has no linear gradient per-rect;
                            // use SUCCESS colour as left-dominant approximation)
                            painter.rect_filled(fill_rect, CornerRadius::same(3), theme::SUCCESS);

                            // Level + zone
                            ui.label(
                                egui::RichText::new("Level 4 \u{00B7} Forest Outpost")
                                    .font(mono_sm.clone())
                                    .color(Color32::from_white_alpha(216)),
                            );
                            ui.label(
                                egui::RichText::new("Quest: Find the lost crate")
                                    .font(mono_sm)
                                    .color(Color32::from_white_alpha(178)),
                            );
                        });
                });

            // ---- Mock FPS counter (top-right of game rect) ----
            let fps_pos = egui::pos2(game_rect.right() - 14.0, game_rect.top() + 12.0);
            egui::Area::new(egui::Id::new("game_fps_hud"))
                .fixed_pos(fps_pos)
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(game_rect);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.label(
                            egui::RichText::new("144 fps")
                                .font(egui::FontId::new(11.0, egui::FontFamily::Monospace))
                                .color(theme::SUCCESS),
                        );
                    });
                });

            // ---- "● PLAYING" badge (bottom-left of game rect) ----
            let badge_pos = egui::pos2(game_rect.left() + 14.0, game_rect.bottom() - 28.0);
            egui::Area::new(egui::Id::new("game_playing_badge"))
                .fixed_pos(badge_pos)
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(game_rect);
                    Frame::NONE
                        .fill(theme::ACCENT)
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("\u{25CF} PLAYING")
                                    .font(egui::FontId::new(10.0, egui::FontFamily::Monospace))
                                    .color(theme::ON_ACCENT)
                                    .strong(),
                            );
                        });
                });
        }

        // ---- UiElement overlay (always when game texture exists) ----
        if self.game_viewport_texture.is_some() {
            let game_rect = self.editor_ctx.game_viewport_rect;
            let painter = ui.painter();

            // Render UI elements, resolving anchors against their Canvas or game_rect
            for eid in self.world.iter_entities() {
                let Some(el) = self.world.get_ui_element(eid) else { continue };
                if !el.visible { continue; }

                // Find the reference rect (Canvas parent → scaled canvas rect, else game_rect)
                let ref_rect = if let Some(parent) = self.world.get_parent(eid) {
                    if let Some(cv) = self.world.get_canvas(parent) {
                        if !cv.visible { continue; }
                        let scale = (game_rect.width() / cv.width).min(game_rect.height() / cv.height).min(1.0);
                        let cw = cv.width * scale;
                        let ch = cv.height * scale;
                        let cx = game_rect.left() + (game_rect.width() - cw) * 0.5;
                        let cy = game_rect.top() + (game_rect.height() - ch) * 0.5;
                        egui::Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(cw, ch))
                    } else {
                        game_rect
                    }
                } else {
                    game_rect
                };

                let anchor_pos = crate::editor::layout::resolve_anchor(el.anchor, ref_rect);
                let pos = anchor_pos + egui::vec2(el.offset[0], el.offset[1]);
                let r = (el.color.x * 255.0) as u8;
                let g = (el.color.y * 255.0) as u8;
                let b = (el.color.z * 255.0) as u8;
                let a = (el.alpha * 255.0) as u8;
                let color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);

                match el.kind {
                    crate::core::UiElementKind::Text => {
                        painter.text(
                            pos,
                            egui::Align2::LEFT_TOP,
                            &el.text,
                            egui::FontId::proportional(el.font_size),
                            color,
                        );
                    }
                    crate::core::UiElementKind::Panel => {
                        let rect = egui::Rect::from_min_size(pos, egui::vec2(el.size[0], el.size[1]));
                        painter.rect_filled(rect, 4.0, color);
                    }
                }
            }
        }
    }
}
