use egui::{Color32, CornerRadius, DragValue, Frame, Margin, Stroke, WidgetText};
use egui_dock::{DockArea, TabViewer};

use super::context::{AssetModal, EditorContext, EditorTab};
use super::{header, theme};
use crate::core::{EntityId, World};
use crate::scripting::GameScript;

// ---- TabViewer ----

/// Holds mutable references needed to render each tab's content.
/// dock_state is extracted via std::mem::take before DockArea::show(),
/// so editor_ctx here does NOT include dock_state (avoids double borrow).
pub struct EditorTabViewer<'a> {
    pub viewport_texture: Option<egui::TextureId>,
    pub game_viewport_texture: Option<egui::TextureId>,
    pub world: &'a mut World,
    pub editor_ctx: &'a mut EditorContext,
    pub scripts: &'a mut Vec<(EntityId, Box<dyn GameScript>)>,
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut EditorTab) -> WidgetText {
        match tab {
            EditorTab::Hierarchy => "\u{1F4CB} Hierarchy".into(),
            EditorTab::Viewport => "\u{1F3AE} Viewport".into(),
            EditorTab::Inspector => "\u{1F50D} Inspector".into(),
            EditorTab::Assets => "\u{1F4C1} Assets".into(),
            EditorTab::Console => "Console".into(),
            EditorTab::GameView => "Game".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut EditorTab) {
        match tab {
            EditorTab::Hierarchy => self.show_hierarchy(ui),
            EditorTab::Viewport => self.show_viewport(ui),
            EditorTab::Inspector => self.show_inspector(ui),
            EditorTab::Assets => self.show_assets(ui),
            EditorTab::Console => self.show_console(ui),
            EditorTab::GameView => self.show_game_view(ui),
        }
    }

    fn closeable(&mut self, _tab: &mut EditorTab) -> bool {
        false
    }
}

// ---- Helpers ----

/// Colored axis label + drag value (Unity-style XYZ inputs)
pub fn axis_drag(ui: &mut egui::Ui, label: &str, color: Color32, value: &mut f32, speed: f32) -> bool {
    Frame::NONE
        .fill(color.gamma_multiply(0.2))
        .corner_radius(CornerRadius { nw: 4, ne: 0, sw: 4, se: 0 })
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).color(color).strong().size(11.0));
        });
    ui.add(DragValue::new(value).speed(speed).max_decimals(3)).changed()
}

/// Replace any non-finite component (NaN / ±Inf) with `fallback`. Used to
/// keep transforms sane after a paste or runaway drag would otherwise poison
/// the model matrix.
pub fn sanitize_vec3(v: &mut glam::Vec3, fallback: f32) {
    if !v.x.is_finite() { v.x = fallback; }
    if !v.y.is_finite() { v.y = fallback; }
    if !v.z.is_finite() { v.z = fallback; }
}

/// Triple colored XYZ drag values laid out horizontally. Returns true if any
/// component changed. Wraps `axis_drag` to remove boilerplate at every Vec3
/// edit site (position, rotation-as-euler, scale, velocity, ...).
pub fn vec3_drag(ui: &mut egui::Ui, value: &mut glam::Vec3, speed: f32) -> bool {
    ui.horizontal(|ui| {
        let cx = axis_drag(ui, "X", theme::AXIS_X, &mut value.x, speed);
        let cy = axis_drag(ui, "Y", theme::AXIS_Y, &mut value.y, speed);
        let cz = axis_drag(ui, "Z", theme::AXIS_Z, &mut value.z, speed);
        cx || cy || cz
    })
    .inner
}

/// Like `vec3_drag` but operates on a [f32; 3] (useful when callers cannot
/// give up an &mut glam::Vec3, e.g. euler angle scratch buffers).
pub fn vec3_drag_array(ui: &mut egui::Ui, value: &mut [f32; 3], speed: f32) -> bool {
    ui.horizontal(|ui| {
        let cx = axis_drag(ui, "X", theme::AXIS_X, &mut value[0], speed);
        let cy = axis_drag(ui, "Y", theme::AXIS_Y, &mut value[1], speed);
        let cz = axis_drag(ui, "Z", theme::AXIS_Z, &mut value[2], speed);
        cx || cy || cz
    })
    .inner
}

/// Component section card — framed card with icon header + optional remove button.
/// Returns `true` if the remove button was clicked.
pub fn component_section(
    ui: &mut egui::Ui,
    id: &str,
    icon: &str,
    title: &str,
    accent: Color32,
    removable: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut remove = false;

    Frame::NONE
        .fill(theme::BG_MANTLE)
        .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(0))
        .show(ui, |ui| {
            // Header bar
            let header_rect = ui.available_rect_before_wrap();
            let header_rect = egui::Rect::from_min_size(
                header_rect.min,
                egui::vec2(ui.available_width(), 26.0),
            );

            // Left accent stripe
            let stripe = egui::Rect::from_min_size(
                header_rect.min,
                egui::vec2(3.0, header_rect.height()),
            );
            ui.painter().rect_filled(stripe, CornerRadius { nw: 6, ne: 0, sw: 0, se: 0 }, accent);

            // Header background
            ui.painter().rect_filled(
                header_rect,
                CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 },
                theme::BG_SURFACE0.gamma_multiply(0.6),
            );
            // Re-draw stripe on top of header bg
            ui.painter().rect_filled(stripe, CornerRadius { nw: 6, ne: 0, sw: 0, se: 0 }, accent);

            let collapse_id = ui.make_persistent_id(id);
            let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(collapse_id, true));

            // Header content
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);
                    // Collapse arrow (animated rotation)
                    let (arrow_rect, arrow_resp) = ui.allocate_exact_size(
                        egui::vec2(12.0, 12.0),
                        egui::Sense::click(),
                    );
                    if arrow_resp.clicked() {
                        open = !open;
                    }
                    // Animated arrow: openness drives rotation from ▶ (0.0) to ▼ (1.0)
                    let openness = ui.ctx().animate_bool_with_time(
                        collapse_id.with("arrow_anim"), open, 0.15,
                    );
                    let center = arrow_rect.center();
                    // Interpolate arrow shape via rotation
                    let angle = openness * std::f32::consts::FRAC_PI_2; // 0 to 90deg
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    // Base shape: right-pointing triangle
                    let pts_base = [
                        egui::vec2(-2.5, -4.0),
                        egui::vec2(3.5, 0.0),
                        egui::vec2(-2.5, 4.0),
                    ];
                    let pts: Vec<egui::Pos2> = pts_base.iter().map(|p| {
                        egui::pos2(
                            center.x + p.x * cos_a - p.y * sin_a,
                            center.y + p.x * sin_a + p.y * cos_a,
                        )
                    }).collect();
                    ui.painter().add(egui::Shape::convex_polygon(
                        pts,
                        theme::TEXT_SECONDARY,
                        Stroke::NONE,
                    ));
                    // Icon (painted colored dot with letter)
                    let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    let ic = icon_rect.center();
                    ui.painter().circle_filled(ic, 5.0, accent.gamma_multiply(0.3));
                    ui.painter().text(
                        ic,
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::monospace(9.0),
                        accent,
                    );
                    // Title (clickable to toggle)
                    let title_resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(title).strong().color(theme::TEXT_PRIMARY).size(12.0),
                        )
                        .selectable(false)
                        .sense(egui::Sense::click()),
                    );
                    if title_resp.clicked() {
                        open = !open;
                    }
                    // Remove button
                    if removable {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(4.0);
                            let btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("x").size(11.0).color(theme::TEXT_DISABLED),
                                )
                                .frame(false),
                            );
                            if btn.hovered() {
                                ui.painter().rect_filled(
                                    btn.rect.expand(2.0),
                                    3.0,
                                    theme::ERROR.gamma_multiply(0.2),
                                );
                            }
                            if btn.on_hover_text("Remove").clicked() {
                                remove = true;
                            }
                        });
                    }
                });
            });

            ui.data_mut(|d| d.insert_persisted(collapse_id, open));

            // Body
            if open {
                ui.add_space(26.0); // skip header
                Frame::NONE
                    .inner_margin(Margin { left: 10, right: 8, top: 4, bottom: 6 })
                    .show(ui, |ui| {
                        body(ui);
                    });
            } else {
                ui.add_space(26.0);
            }
        });

    ui.add_space(4.0);
    remove
}

/// Property row with fixed-width label for alignment.
/// Label column is 116px wide (spec: grid 116px 1fr, gap 6px).
pub fn property_row(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(116.0, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .color(theme::TEXT_SECONDARY)
                        .size(11.0),
                );
            },
        );
        add_contents(ui);
    });
}

/// Custom slider matching the design mockup: 4px track (BG_SURFACE1 bg, ACCENT
/// fill), 10px knob (TEXT_PRIMARY fill + 2px ACCENT border).
/// Returns `true` if the value changed.
pub fn theme_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let (min, max) = (*range.start(), *range.end());
    let span = (max - min).max(f32::EPSILON);
    let t = ((*value - min) / span).clamp(0.0, 1.0);

    // Allocate the full available width, 16px tall (knob needs ~10px vertical)
    let desired = egui::vec2(ui.available_width(), 16.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    // Track geometry: 4px tall, vertically centred
    let track_y = rect.center().y;
    let track_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), track_y - 2.0),
        egui::pos2(rect.right() - 40.0, track_y + 2.0),
    );
    let knob_x = track_rect.left() + t * track_rect.width();

    // Interaction: update value on drag/click within the track area
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let new_t = ((pos.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
            let new_val = min + new_t * span;
            if (*value - new_val).abs() > f32::EPSILON {
                *value = new_val;
                changed = true;
            }
        }
    }

    let painter = ui.painter();

    // Track background
    painter.rect_filled(track_rect, egui::CornerRadius::same(2), theme::BG_SURFACE1);
    // Track fill (accent)
    let fill_rect = egui::Rect::from_min_max(
        track_rect.min,
        egui::pos2(knob_x.min(track_rect.right()), track_rect.max.y),
    );
    painter.rect_filled(fill_rect, egui::CornerRadius::same(2), theme::ACCENT);

    // Knob: 10px circle, TEXT_PRIMARY fill, 2px ACCENT stroke
    painter.circle(
        egui::pos2(knob_x, track_y),
        5.0,
        theme::TEXT_PRIMARY,
        egui::Stroke::new(2.0, theme::ACCENT),
    );

    // Value label on the right (monospace, 11px)
    let label_rect = egui::Rect::from_min_max(
        egui::pos2(track_rect.right() + 4.0, rect.top()),
        egui::pos2(rect.right(), rect.bottom()),
    );
    painter.text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.2}", *value),
        egui::FontId::monospace(10.5),
        theme::TEXT_SECONDARY,
    );

    changed
}

/// Color swatch: small framed rect showing the color + egui color picker popup.
/// Returns `true` if the color changed.
pub fn color_swatch(ui: &mut egui::Ui, rgb: &mut [f32; 3]) -> bool {
    let swatch_size = egui::vec2(28.0, 16.0);
    let color = egui::Color32::from_rgb(
        (rgb[0] * 255.0) as u8,
        (rgb[1] * 255.0) as u8,
        (rgb[2] * 255.0) as u8,
    );

    // Outer frame: BG_SURFACE0 bg + hairline border
    let frame_resp = egui::Frame::NONE
        .fill(theme::BG_SURFACE0)
        .stroke(egui::Stroke::new(1.0, theme::BG_SURFACE1))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(swatch_size, egui::Sense::click());
            ui.painter().rect_filled(rect, egui::CornerRadius::same(2), color);
            resp
        });

    let mut changed = false;
    // Show egui's built-in color picker as a popup on click
    egui::Popup::from_toggle_button_response(&frame_resp.inner)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|popup_ui: &mut egui::Ui| {
            popup_ui.set_min_width(220.0);
            if popup_ui.color_edit_button_rgb(rgb).changed() {
                changed = true;
            }
        });

    changed
}

/// Texture slot card: placeholder square + filename + remove/browse
pub fn texture_slot(
    ui: &mut egui::Ui,
    label: &str,
    path: &Option<String>,
    placeholder_color: Color32,
) -> TextureSlotAction {
    let mut action = TextureSlotAction::None;

    ui.add_space(2.0);
    ui.label(egui::RichText::new(label).color(theme::TEXT_DISABLED).small());

    ui.horizontal(|ui| {
        // Placeholder square
        let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
        if path.is_some() {
            ui.painter().rect_filled(rect, 3.0, placeholder_color);
        } else {
            // Checkerboard pattern for empty
            let half = rect.size() / 2.0;
            let c1 = theme::BG_SURFACE0;
            let c2 = theme::BG_SURFACE1;
            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, half), 0.0, c1);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min + egui::vec2(half.x, 0.0), half), 0.0, c2,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min + egui::vec2(0.0, half.y), half), 0.0, c2,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min + half, half), 0.0, c1,
            );
            ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, theme::BG_SURFACE1), egui::StrokeKind::Outside);
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if let Some(ref p) = path {
                    let display = p.rsplit('/').next().unwrap_or(p);
                    ui.label(egui::RichText::new(display).color(theme::TEXT_PRIMARY).size(11.0));
                    let x_btn = ui.add(
                        egui::Button::new(egui::RichText::new("x").size(10.0).color(theme::ERROR))
                            .frame(false),
                    );
                    if x_btn.on_hover_text("Remove").clicked() {
                        action = TextureSlotAction::Remove;
                    }
                } else {
                    ui.label(egui::RichText::new("None").color(theme::TEXT_DISABLED).size(11.0));
                }
            });
        });
    });

    action
}

#[derive(PartialEq)]
pub enum TextureSlotAction {
    None,
    Remove,
}

fn deduplicate_scene_name(base: &str) -> String {
    let path = format!("assets/scenes/{}.ron", base);
    if !std::path::Path::new(&path).exists() {
        return base.to_string();
    }
    for i in 1.. {
        let candidate = format!("{} ({})", base, i);
        let cpath = format!("assets/scenes/{}.ron", candidate);
        if !std::path::Path::new(&cpath).exists() {
            return candidate;
        }
    }
    base.to_string()
}

/// Resolve a UiAnchor to a position within a given rect.
pub fn resolve_anchor(anchor: crate::core::UiAnchor, rect: egui::Rect) -> egui::Pos2 {
    use crate::core::UiAnchor::*;
    match anchor {
        TopLeft => rect.left_top(),
        TopCenter => egui::pos2(rect.center().x, rect.top()),
        TopRight => rect.right_top(),
        CenterLeft => egui::pos2(rect.left(), rect.center().y),
        Center => rect.center(),
        CenterRight => egui::pos2(rect.right(), rect.center().y),
        BottomLeft => rect.left_bottom(),
        BottomCenter => egui::pos2(rect.center().x, rect.bottom()),
        BottomRight => rect.right_bottom(),
    }
}

pub fn entity_icon(world: &World, id: EntityId) -> (&'static str, Color32) {
    use crate::editor::theme;
    if world.get_canvas(id).is_some() {
        ("\u{1F5BC}", theme::MAUVE) // canvas frame
    } else if world.get_camera(id).is_some() {
        ("\u{1F3A5}", theme::SKY) // camera
    } else if world.get_light(id).is_some() {
        ("\u{2600}", Color32::from_rgb(0xF0, 0xC0, 0x40)) // sun (warmer yellow than WARNING)
    } else if world.get_ui_element(id).is_some() {
        ("\u{1F5B5}", theme::MAUVE) // UI icon
    } else if world.get_audio_source(id).is_some() {
        ("\u{1F50A}", theme::WARNING) // speaker
    } else if world.get_mesh_renderer(id).is_some() {
        ("\u{25A0}", Color32::from_rgb(0x4B, 0x8B, 0xBE)) // blue square (kept — distinct from accents)
    } else {
        ("\u{25CB}", Color32::from_rgb(0x80, 0x80, 0x80)) // gray circle (neutral)
    }
}

/// Small mid-dot separator used between status bar items.
fn status_dot_sep(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("\u{00B7}")
            .color(theme::TEXT_DISABLED.gamma_multiply(0.6))
            .monospace()
            .size(10.5),
    );
}

// ---- Main Layout Entry Point ----

pub struct EditorLayout;

impl EditorLayout {
    pub fn show(
        ctx: &egui::Context,
        viewport_texture: Option<egui::TextureId>,
        game_viewport_texture: Option<egui::TextureId>,
        world: &mut World,
        editor_ctx: &mut EditorContext,
        scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
    ) {
        // Header stays OUTSIDE the dock area (TopBottomPanel)
        header::show_header(ctx, editor_ctx);

        // Status bar — must be registered BEFORE the DockArea (same as the
        // header) so the dock fills the remaining central space.
        Self::show_status_bar(ctx, editor_ctx);

        // Extract dock_state to separate borrows (same pattern as scripts)
        let mut dock_state = std::mem::replace(
            &mut editor_ctx.dock_state,
            egui_dock::DockState::new(vec![]),
        );

        let mut viewer = EditorTabViewer {
            viewport_texture,
            game_viewport_texture,
            world,
            editor_ctx,
            scripts,
        };

        DockArea::new(&mut dock_state)
            .show_close_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .tab_context_menus(false)
            .style(theme::dock_style(ctx.style().as_ref()))
            .show(ctx, &mut viewer);

        // Put dock_state back
        editor_ctx.dock_state = dock_state;

        // Asset modal (rendered at egui::Context level, not inside dock)
        Self::show_asset_modal(ctx, editor_ctx);
        Self::show_confirm_delete_asset_modal(ctx, editor_ctx);
    }

    /// Bottom status bar (mockup's 24px footer). Registered before the dock so
    /// it reserves space at the bottom of the window.
    fn show_status_bar(ctx: &egui::Context, editor_ctx: &EditorContext) {
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(24.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(theme::BG_MANTLE)
                    .inner_margin(Margin::symmetric(10, 0))
                    .stroke(Stroke::new(1.0, theme::BG_SURFACE0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // Live indicator dot
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 3.0, theme::SUCCESS);

                    let state_label = if editor_ctx.play_mode {
                        "Editor \u{2022} Play mode"
                    } else {
                        "Editor \u{2022} idle"
                    };
                    ui.label(
                        egui::RichText::new(state_label)
                            .color(theme::TEXT_DISABLED)
                            .monospace()
                            .size(10.5),
                    );

                    status_dot_sep(ui);
                    ui.label(
                        egui::RichText::new(format!(
                            "Entities {}",
                            editor_ctx.entity_count
                        ))
                        .color(theme::TEXT_DISABLED)
                        .monospace()
                        .size(10.5),
                    );

                    status_dot_sep(ui);
                    let frame_ms = if editor_ctx.fps > 0.0 {
                        1000.0 / editor_ctx.fps
                    } else {
                        0.0
                    };
                    ui.label(
                        egui::RichText::new(format!("Frame {:.1} ms", frame_ms))
                            .color(theme::TEXT_DISABLED)
                            .monospace()
                            .size(10.5),
                    );

                    // Right side: build badge + git status
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new("main \u{2022} clean")
                                    .color(theme::TEXT_DISABLED)
                                    .monospace()
                                    .size(10.5),
                            );
                            ui.add_space(8.0);
                            Frame::NONE
                                .fill(theme::ACCENT_SOFT)
                                .stroke(Stroke::new(1.0, theme::ACCENT))
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(Margin::symmetric(6, 1))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("built with Rust \u{2022} wgpu")
                                            .color(theme::ACCENT)
                                            .monospace()
                                            .size(10.0),
                                    );
                                });
                        },
                    );
                });
            });
    }

    /// Confirmation gate for asset deletion. Unlike entity deletion (captured
    /// by the undo stack), removing a file/folder is irreversible.
    fn show_confirm_delete_asset_modal(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
        let Some(path) = editor_ctx.confirm_delete_asset.clone() else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("this item")
            .to_string();

        let mut confirmed = false;
        let mut cancelled = false;

        egui::Window::new("Delete asset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!("Delete \u{201C}{name}\u{201D}?"))
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("This permanently removes it from disk and cannot be undone.")
                        .color(theme::TEXT_DISABLED)
                        .small(),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("Delete").color(theme::ERROR))
                        .clicked()
                    {
                        confirmed = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
                // Esc cancels, Enter does NOT confirm (destructive — require
                // an explicit click on Delete).
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancelled = true;
                }
            });

        if confirmed {
            editor_ctx.pending_delete_asset = Some(path);
            editor_ctx.confirm_delete_asset = None;
        } else if cancelled {
            editor_ctx.confirm_delete_asset = None;
        }
    }

    fn show_asset_modal(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
        let modal = editor_ctx.asset_modal.clone();
        if let Some(modal_state) = modal {
            let (title, mut name, kind) = match modal_state {
                AssetModal::NewFolder { name } => ("New Folder", name, 0),
                AssetModal::NewScript { name } => ("New Script", name, 1),
                AssetModal::NewScene { name } => ("New Scene", name, 2),
            };

            let mut confirmed = false;
            let mut cancelled = false;

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let resp = ui.text_edit_singleline(&mut name);
                        if resp.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirmed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirmed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });

            if confirmed && !name.trim().is_empty() {
                let sanitized: String = name
                    .trim()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.' || *c == ' ')
                    .collect();
                if !sanitized.is_empty() {
                    match kind {
                        0 => editor_ctx.pending_create_folder = Some(sanitized),
                        1 => editor_ctx.pending_create_script = Some(sanitized),
                        2 => {
                            let final_name = deduplicate_scene_name(&sanitized);
                            editor_ctx.pending_new_scene_name = Some(final_name);
                        }
                        _ => {}
                    }
                }
                editor_ctx.asset_modal = None;
            } else if cancelled {
                editor_ctx.asset_modal = None;
            } else {
                editor_ctx.asset_modal = Some(match kind {
                    0 => AssetModal::NewFolder { name },
                    1 => AssetModal::NewScript { name },
                    _ => AssetModal::NewScene { name },
                });
            }
        }
    }
}
