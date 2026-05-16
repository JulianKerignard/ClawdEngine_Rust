use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

use crate::editor::console::LogLevel;
use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_console(&mut self, ui: &mut egui::Ui) {
        let (error_count, warn_count, info_count, debug_count) =
            self.editor_ctx.log_buffer.counts();

        // ---- Toolbar ----
        Frame::NONE
            .fill(theme::BG_MANTLE)
            .inner_margin(Margin::symmetric(6, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Clear button
                    let clear_btn = egui::Button::new(
                        egui::RichText::new("Clear")
                            .font(egui::FontId::monospace(11.0))
                            .color(theme::TEXT_SECONDARY),
                    )
                    .fill(theme::BG_SURFACE0)
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, theme::BG_SURFACE1));

                    if ui.add(clear_btn).clicked() {
                        self.editor_ctx.log_buffer.clear();
                    }

                    ui.add_space(8.0);

                    filter_toggle(
                        ui, "● Error", error_count, theme::ERROR,
                        &mut self.editor_ctx.console_filter_error,
                    );
                    filter_toggle(
                        ui, "▲ Warn", warn_count, theme::WARNING,
                        &mut self.editor_ctx.console_filter_warn,
                    );
                    filter_toggle(
                        ui, "● Info", info_count, theme::SUCCESS,
                        &mut self.editor_ctx.console_filter_info,
                    );
                    filter_toggle(
                        ui, "■ Debug", debug_count, theme::TEXT_DISABLED,
                        &mut self.editor_ctx.console_filter_debug,
                    );
                });
            });

        // Hairline separator
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, theme::BG_SURFACE0),
        );

        // ---- Log entries ----
        let filter_error = self.editor_ctx.console_filter_error;
        let filter_warn  = self.editor_ctx.console_filter_warn;
        let filter_info  = self.editor_ctx.console_filter_info;
        let filter_debug = self.editor_ctx.console_filter_debug;
        let auto_scroll  = self.editor_ctx.console_auto_scroll;

        let scroll = egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(auto_scroll);

        scroll.show(ui, |ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0;

            let total_visible = self.editor_ctx.log_buffer.with_entries(|entries| {
                let mut visible_idx = 0u32;
                for entry in entries {
                    let show = match entry.level {
                        LogLevel::Error => filter_error,
                        LogLevel::Warn  => filter_warn,
                        LogLevel::Info  => filter_info,
                        LogLevel::Debug => filter_debug,
                    };
                    if !show {
                        continue;
                    }

                    // Severity: icon glyph + message color
                    let (icon, msg_color) = match entry.level {
                        LogLevel::Error => ("●", theme::ERROR),
                        LogLevel::Warn  => ("▲", theme::WARNING),
                        LogLevel::Info  => ("●", theme::TEXT_PRIMARY),
                        LogLevel::Debug => ("■", theme::TEXT_DISABLED),
                    };

                    // Alternating row background
                    let row_bg = if visible_idx % 2 == 0 {
                        Color32::TRANSPARENT
                    } else {
                        Color32::from_white_alpha(4)
                    };
                    visible_idx += 1;

                    let row_height = 20.0;
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::hover(),
                    );

                    // Row background
                    ui.painter().rect_filled(rect, 0.0, row_bg);
                    // Hover highlight
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 0.0, theme::BG_SURFACE0);
                    }

                    let cy  = rect.center().y;
                    let pad = 10.0;
                    let mut x = rect.left() + pad;

                    // [icon  18px] severity glyph, colored
                    ui.painter().text(
                        egui::pos2(x + 9.0, cy),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::monospace(10.0),
                        msg_color,
                    );
                    x += 18.0;

                    // [time  64px] monospace, TEXT_DISABLED
                    let minutes = (entry.timestamp_secs / 60.0) as u32;
                    let secs    = entry.timestamp_secs % 60.0;
                    let ts      = format!("{:02}:{:05.2}", minutes, secs);
                    ui.painter().text(
                        egui::pos2(x, cy),
                        egui::Align2::LEFT_CENTER,
                        &ts,
                        egui::FontId::monospace(10.0),
                        theme::TEXT_DISABLED,
                    );
                    x += 64.0 + 8.0;

                    // [message  remaining width] colored by severity
                    let msg_width = (rect.right() - pad - x).max(0.0);
                    if msg_width > 0.0 {
                        // Clip message text to available width (no wrapping)
                        let galley = ui.painter().layout_no_wrap(
                            entry.message.clone(),
                            egui::FontId::monospace(11.0),
                            msg_color,
                        );
                        let clip_rect = egui::Rect::from_min_size(
                            egui::pos2(x, cy - row_height / 2.0),
                            egui::vec2(msg_width, row_height),
                        );
                        ui.painter().with_clip_rect(clip_rect).galley(
                            egui::pos2(x, cy - galley.size().y / 2.0),
                            galley,
                            msg_color,
                        );
                    }

                    // Bottom hairline
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom(),
                        Stroke::new(1.0, Color32::from_black_alpha(20)),
                    );
                }
                visible_idx
            });

            if total_visible == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("No log messages")
                            .color(theme::TEXT_DISABLED),
                    );
                });
            }
        });
    }
}

fn filter_toggle(
    ui: &mut egui::Ui,
    label: &str,
    count: u32,
    color: Color32,
    enabled: &mut bool,
) {
    let text = if count > 0 {
        format!("{} {}", label, count)
    } else {
        label.to_string()
    };

    let btn_color = if *enabled { color } else { theme::TEXT_DISABLED };
    let fill = if *enabled {
        color.gamma_multiply(0.15)
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if *enabled {
        Stroke::new(1.0, color.gamma_multiply(0.3))
    } else {
        Stroke::NONE
    };

    let btn = egui::Button::new(
        egui::RichText::new(&text)
            .font(egui::FontId::monospace(11.0))
            .color(btn_color),
    )
    .fill(fill)
    .corner_radius(CornerRadius::same(4))
    .stroke(stroke);

    if ui.add(btn).clicked() {
        *enabled = !*enabled;
    }
}
