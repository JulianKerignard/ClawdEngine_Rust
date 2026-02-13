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

/// Property row with fixed-width label for alignment
pub fn property_row(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(72.0, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .color(theme::TEXT_SECONDARY)
                        .size(12.0),
                );
            },
        );
        add_contents(ui);
    });
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
            let browse = ui.small_button("Browse...");
            if browse.clicked() {
                action = TextureSlotAction::BrowseClicked;
            }
            // Return browse response for popup attachment
            ui.data_mut(|d| d.insert_temp(egui::Id::new(format!("browse_resp_{}", label)), browse));
        });
    });

    action
}

#[derive(PartialEq)]
pub enum TextureSlotAction {
    None,
    Remove,
    BrowseClicked,
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
    if world.get_canvas(id).is_some() {
        ("\u{1F5BC}", Color32::from_rgb(0xCB, 0xA6, 0xF7)) // mauve canvas frame
    } else if world.get_camera(id).is_some() {
        ("\u{1F3A5}", Color32::from_rgb(0x87, 0xDB, 0xEB)) // sky blue camera
    } else if world.get_light(id).is_some() {
        ("\u{2600}", Color32::from_rgb(0xF0, 0xC0, 0x40)) // yellow sun
    } else if world.get_ui_element(id).is_some() {
        ("\u{1F5B5}", Color32::from_rgb(0xCB, 0xA6, 0xF7)) // mauve UI icon
    } else if world.get_audio_source(id).is_some() {
        ("\u{1F50A}", Color32::from_rgb(0xF9, 0xE2, 0xAF)) // yellow speaker
    } else if world.get_mesh_renderer(id).is_some() {
        ("\u{25A0}", Color32::from_rgb(0x4B, 0x8B, 0xBE)) // blue square
    } else {
        ("\u{25CB}", Color32::from_rgb(0x80, 0x80, 0x80)) // gray circle
    }
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
        // Fullscreen game mode: skip all editor UI
        if editor_ctx.fullscreen_game {
            Self::show_fullscreen_game(ctx, game_viewport_texture, world, editor_ctx, scripts);
            return;
        }

        // Header stays OUTSIDE the dock area (TopBottomPanel)
        header::show_header(ctx, editor_ctx);

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
    }

    fn show_fullscreen_game(
        ctx: &egui::Context,
        game_viewport_texture: Option<egui::TextureId>,
        world: &World,
        editor_ctx: &mut EditorContext,
        scripts: &mut Vec<(EntityId, Box<dyn GameScript>)>,
    ) {
        // Keep game viewport texture alive
        editor_ctx.game_view_visible = true;

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                if let Some(tex_id) = game_viewport_texture {
                    let available = ui.available_size();
                    ui.image(egui::load::SizedTexture::new(tex_id, available));
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No main camera in scene.\nPress ESC to return to editor.")
                                .color(Color32::WHITE)
                                .size(18.0),
                        );
                    });
                }
                editor_ctx.game_viewport_rect = ui.min_rect();
            });

        let game_rect = editor_ctx.game_viewport_rect;

        // HUD: script game_ui overlays (play mode only)
        if editor_ctx.play_mode && game_viewport_texture.is_some() {
            let mut scripts_taken = std::mem::take(scripts);
            for (eid, script) in &mut scripts_taken {
                let area_id = egui::Id::new("fs_hud").with(eid.index);
                egui::Area::new(area_id)
                    .fixed_pos(game_rect.left_top())
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        ui.set_clip_rect(game_rect);
                        ui.set_max_size(game_rect.size());
                        script.game_ui(ui);
                    });
            }
            *scripts = scripts_taken;
        }

        // HUD: UiElement/Canvas overlays
        if game_viewport_texture.is_some() {
            egui::Area::new(egui::Id::new("fs_ui_elements"))
                .fixed_pos(game_rect.left_top())
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.set_clip_rect(game_rect);
                    let painter = ui.painter();

                    for eid in world.iter_entities() {
                        let Some(el) = world.get_ui_element(eid) else { continue };
                        if !el.visible { continue; }

                        let ref_rect = if let Some(parent) = world.get_parent(eid) {
                            if let Some(cv) = world.get_canvas(parent) {
                                if !cv.visible { continue; }
                                let scale = (game_rect.width() / cv.width)
                                    .min(game_rect.height() / cv.height)
                                    .min(1.0);
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

                        let anchor_pos = resolve_anchor(el.anchor, ref_rect);
                        let pos = anchor_pos + egui::vec2(el.offset[0], el.offset[1]);
                        let r = (el.color.x * 255.0) as u8;
                        let g = (el.color.y * 255.0) as u8;
                        let b = (el.color.z * 255.0) as u8;
                        let a = (el.alpha * 255.0) as u8;
                        let color = Color32::from_rgba_unmultiplied(r, g, b, a);

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
                                let rect = egui::Rect::from_min_size(
                                    pos,
                                    egui::vec2(el.size[0], el.size[1]),
                                );
                                painter.rect_filled(rect, 4.0, color);
                            }
                        }
                    }
                });
        }

        // ESC hint overlay
        egui::Area::new(egui::Id::new("fs_esc_hint"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("ESC to exit")
                        .color(Color32::from_white_alpha(80))
                        .size(11.0),
                );
            });
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
