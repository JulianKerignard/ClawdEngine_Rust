use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_game_view(&mut self, ui: &mut egui::Ui) {
        self.editor_ctx.game_view_visible = true;

        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            theme::BG_CRUST,
        );

        if let Some(tex_id) = self.game_viewport_texture {
            let available = ui.available_size();
            ui.image(egui::load::SizedTexture::new(tex_id, available));
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
