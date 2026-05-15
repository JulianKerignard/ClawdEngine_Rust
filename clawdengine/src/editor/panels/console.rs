use egui::{Color32, CornerRadius, Stroke};

use crate::editor::console::LogLevel;
use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_console(&mut self, ui: &mut egui::Ui) {
        let (error_count, warn_count, info_count) = self.editor_ctx.log_buffer.counts();

        // ---- Toolbar ----
        ui.horizontal(|ui| {
            ui.strong("Console");
            ui.add_space(8.0);

            if ui.small_button("Clear").clicked() {
                self.editor_ctx.log_buffer.clear();
            }

            ui.add_space(12.0);

            filter_toggle(ui, "Error", error_count, theme::ERROR, &mut self.editor_ctx.console_filter_error);
            filter_toggle(ui, "Warn", warn_count, theme::WARNING, &mut self.editor_ctx.console_filter_warn);
            filter_toggle(ui, "Info", info_count, theme::SUCCESS, &mut self.editor_ctx.console_filter_info);
            filter_toggle(ui, "Debug", 0, theme::TEXT_DISABLED, &mut self.editor_ctx.console_filter_debug);
        });

        ui.separator();

        // ---- Log entries ----
        let filter_error = self.editor_ctx.console_filter_error;
        let filter_warn = self.editor_ctx.console_filter_warn;
        let filter_info = self.editor_ctx.console_filter_info;
        let filter_debug = self.editor_ctx.console_filter_debug;
        let auto_scroll = self.editor_ctx.console_auto_scroll;

        let scroll = egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(auto_scroll);

        scroll.show(ui, |ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0;

            // Borrow log entries directly (no clone): the lock is held for the
            // duration of the closure which only touches egui painter primitives.
            let total_visible = self.editor_ctx.log_buffer.with_entries(|entries| {
                let mut visible_idx = 0u32;
                for entry in entries {
                    let show = match entry.level {
                        LogLevel::Error => filter_error,
                        LogLevel::Warn => filter_warn,
                        LogLevel::Info => filter_info,
                        LogLevel::Debug => filter_debug,
                    };
                    if !show {
                        continue;
                    }

                    let (level_str, level_color) = match entry.level {
                        LogLevel::Error => ("[ERR]", theme::ERROR),
                        LogLevel::Warn => ("[WRN]", theme::WARNING),
                        LogLevel::Info => ("[INF]", theme::SUCCESS),
                        LogLevel::Debug => ("[DBG]", theme::TEXT_DISABLED),
                    };

                    let row_bg = if visible_idx % 2 == 0 {
                        Color32::TRANSPARENT
                    } else {
                        Color32::from_white_alpha(4)
                    };
                    visible_idx += 1;

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 20.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, row_bg);

                    let minutes = (entry.timestamp_secs / 60.0) as u32;
                    let secs = entry.timestamp_secs % 60.0;
                    let ts = format!("{:02}:{:05.2}", minutes, secs);

                    let mut x = rect.left() + 4.0;
                    let cy = rect.center().y;

                    ui.painter().text(
                        egui::pos2(x, cy),
                        egui::Align2::LEFT_CENTER,
                        &ts,
                        egui::FontId::monospace(11.0),
                        theme::TEXT_DISABLED,
                    );
                    x += 64.0;

                    ui.painter().text(
                        egui::pos2(x, cy),
                        egui::Align2::LEFT_CENTER,
                        level_str,
                        egui::FontId::monospace(11.0),
                        level_color,
                    );
                    x += 42.0;

                    ui.painter().text(
                        egui::pos2(x, cy),
                        egui::Align2::LEFT_CENTER,
                        &entry.message,
                        egui::FontId::proportional(12.0),
                        theme::TEXT_PRIMARY,
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

    let btn = egui::Button::new(
        egui::RichText::new(&text).color(btn_color).size(11.0),
    )
    .fill(fill)
    .corner_radius(CornerRadius::same(4))
    .stroke(if *enabled {
        Stroke::new(1.0, color.gamma_multiply(0.3))
    } else {
        Stroke::NONE
    });

    if ui.add(btn).clicked() {
        *enabled = !*enabled;
    }
}
