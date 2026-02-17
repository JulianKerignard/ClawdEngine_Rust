use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};

use crate::core::{AnimationState, TransitionSource};
use crate::editor::layout::EditorTabViewer;
use crate::editor::theme;

// ---- Constants ----

const NODE_WIDTH: f32 = 160.0;
const NODE_HEIGHT: f32 = 50.0;
const GRID_SIZE: f32 = 20.0;
const NODE_CORNER_RADIUS: f32 = 8.0;
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 2.0;

// Catppuccin Mocha palette colors for the graph
const STATE_BG: Color32 = Color32::from_rgb(0x45, 0x47, 0x5A);          // surface1
const STATE_ACTIVE_COLOR: Color32 = Color32::from_rgb(0xA6, 0xE3, 0xA1); // green
const STATE_DEFAULT_COLOR: Color32 = Color32::from_rgb(0x89, 0xB4, 0xFA); // blue
const STATE_BORDER: Color32 = Color32::from_rgb(0x58, 0x5B, 0x70);       // surface2
const TRANSITION_COLOR: Color32 = Color32::from_rgb(0xF5, 0xC2, 0xE7);   // pink
const GRID_COLOR: Color32 = Color32::from_rgb(0x24, 0x25, 0x37);         // slightly lighter than crust
const CANVAS_BG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x2E);         // base
const ANY_STATE_COLOR: Color32 = Color32::from_rgb(0xF9, 0xE2, 0xAF);   // yellow
const CLIP_TEXT_COLOR: Color32 = Color32::from_rgb(0xA6, 0xAD, 0xC8);   // subtext0

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_animator_graph(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.strong("\u{1F3AC} Animator Graph");
        });
        ui.separator();

        // Check selection
        if self.editor_ctx.selected_entities.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select an entity with an Animator Controller")
                        .color(theme::TEXT_DISABLED),
                );
            });
            return;
        }

        let eid = self.editor_ctx.selected_entities[0];

        // Get the SkeletalAnimator for this entity
        let (controller_id, current_state_idx, is_blending) = {
            let sa = match self.world.get_skeletal_animator(eid) {
                Some(sa) if sa.controller_id.is_some() => sa,
                _ => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No Animator Controller assigned")
                                .color(theme::TEXT_DISABLED),
                        );
                    });
                    return;
                }
            };
            let ctrl_id = sa.controller_id.unwrap();
            let (cur, blending) = if let Some(ref cs) = sa.controller_state {
                (cs.current_state, cs.is_blending)
            } else {
                (0, false)
            };
            (ctrl_id, cur, blending)
        };

        // Get the controller definition from the store
        let controller = match self.animator_controller_store.get(controller_id) {
            Some(c) => c.clone(),
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Controller not found in store")
                            .color(theme::TEXT_DISABLED),
                    );
                });
                return;
            }
        };

        // Controller name header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Controller: {}", controller.name))
                    .color(theme::TEXT_SECONDARY)
                    .size(11.0),
            );
            ui.label(
                egui::RichText::new(format!(
                    "  |  States: {}  |  Transitions: {}",
                    controller.states.len(),
                    controller.transitions.len()
                ))
                .color(theme::TEXT_DISABLED)
                .size(11.0),
            );
        });
        ui.add_space(2.0);

        // ---- Canvas ----
        let (response, mut painter) =
            ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas_rect = response.rect;

        // Read graph state
        let pan = self.editor_ctx.animator_graph_pan;
        let zoom = self.editor_ctx.animator_graph_zoom;

        // Background
        painter.rect_filled(canvas_rect, 0.0, CANVAS_BG);

        // Clip to canvas
        painter.set_clip_rect(canvas_rect);

        // Draw grid
        draw_grid(&painter, canvas_rect, pan, GRID_SIZE * zoom, GRID_COLOR);

        // Compute node positions (use stored overrides or auto-layout)
        let node_positions: Vec<Pos2> = controller
            .states
            .iter()
            .enumerate()
            .map(|(i, state)| {
                if let Some(&(x, y)) = self.editor_ctx.animator_graph_positions.get(&i) {
                    Pos2::new(x, y)
                } else if state.position != (0.0, 0.0) {
                    Pos2::new(state.position.0, state.position.1)
                } else {
                    // Auto-layout: arrange in a grid
                    let cols = 3;
                    let col = i % cols;
                    let row = i / cols;
                    let x = 80.0 + col as f32 * 220.0;
                    let y = 60.0 + row as f32 * 100.0;
                    Pos2::new(x, y)
                }
            })
            .collect();

        // Convert node positions to screen coordinates
        let screen_positions: Vec<Pos2> = node_positions
            .iter()
            .map(|p| {
                Pos2::new(
                    canvas_rect.min.x + p.x * zoom + pan.x,
                    canvas_rect.min.y + p.y * zoom + pan.y,
                )
            })
            .collect();

        let scaled_w = NODE_WIDTH * zoom;
        let scaled_h = NODE_HEIGHT * zoom;

        // Compute node rects
        let node_rects: Vec<Rect> = screen_positions
            .iter()
            .map(|&pos| Rect::from_min_size(pos, Vec2::new(scaled_w, scaled_h)))
            .collect();

        // ---- Draw transitions (arrows) ----
        for transition in &controller.transitions {
            let to_idx = transition.to_state;
            if to_idx >= node_rects.len() {
                continue;
            }
            let to_rect = node_rects[to_idx];

            match &transition.from_state {
                TransitionSource::State(from_idx) => {
                    if *from_idx < node_rects.len() {
                        let from_rect = node_rects[*from_idx];
                        draw_transition_arrow(
                            &painter,
                            from_rect,
                            to_rect,
                            TRANSITION_COLOR,
                            zoom,
                        );
                    }
                }
                TransitionSource::AnyState => {
                    // Draw from a special "Any State" indicator above the target
                    let any_pos = Pos2::new(
                        to_rect.center().x,
                        to_rect.min.y - 40.0 * zoom,
                    );
                    // Small diamond for "Any State"
                    let diamond_size = 8.0 * zoom;
                    let diamond = vec![
                        Pos2::new(any_pos.x, any_pos.y - diamond_size),
                        Pos2::new(any_pos.x + diamond_size, any_pos.y),
                        Pos2::new(any_pos.x, any_pos.y + diamond_size),
                        Pos2::new(any_pos.x - diamond_size, any_pos.y),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        diamond,
                        ANY_STATE_COLOR.gamma_multiply(0.4),
                        Stroke::new(1.5 * zoom, ANY_STATE_COLOR),
                    ));
                    painter.text(
                        any_pos + Vec2::new(diamond_size + 4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "Any",
                        egui::FontId::proportional(9.0 * zoom),
                        ANY_STATE_COLOR,
                    );
                    // Arrow from diamond to target
                    let start = Pos2::new(any_pos.x, any_pos.y + diamond_size);
                    let end = Pos2::new(to_rect.center().x, to_rect.min.y);
                    painter.line_segment(
                        [start, end],
                        Stroke::new(1.5 * zoom, ANY_STATE_COLOR.gamma_multiply(0.7)),
                    );
                    // Arrowhead
                    let dir = (end - start).normalized();
                    let arrow_size = 6.0 * zoom;
                    let perp = Vec2::new(-dir.y, dir.x);
                    let left = end - dir * arrow_size + perp * arrow_size * 0.5;
                    let right = end - dir * arrow_size - perp * arrow_size * 0.5;
                    painter.add(egui::Shape::convex_polygon(
                        vec![end, left, right],
                        ANY_STATE_COLOR.gamma_multiply(0.7),
                        Stroke::NONE,
                    ));
                }
            }
        }

        // ---- Draw state nodes ----
        let play_mode = self.editor_ctx.play_mode;
        for (i, state) in controller.states.iter().enumerate() {
            if i >= node_rects.len() {
                break;
            }
            let rect = node_rects[i];
            let is_current = play_mode && i == current_state_idx;
            let is_default = i == controller.default_state;

            draw_state_node(
                &painter,
                state,
                rect,
                is_current,
                is_default,
                is_blending && is_current,
                zoom,
            );
        }

        // ---- Draw "Entry" arrow to default state ----
        if controller.default_state < node_rects.len() {
            let default_rect = node_rects[controller.default_state];
            let entry_start = Pos2::new(default_rect.min.x - 40.0 * zoom, default_rect.center().y);
            let entry_end = Pos2::new(default_rect.min.x, default_rect.center().y);

            // "Entry" label
            painter.text(
                entry_start - Vec2::new(4.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                "Entry",
                egui::FontId::proportional(10.0 * zoom),
                STATE_DEFAULT_COLOR,
            );

            // Arrow
            painter.line_segment(
                [entry_start, entry_end],
                Stroke::new(2.0 * zoom, STATE_DEFAULT_COLOR),
            );
            let arrow_size = 7.0 * zoom;
            let tip = entry_end;
            let left = Pos2::new(tip.x - arrow_size, tip.y - arrow_size * 0.5);
            let right = Pos2::new(tip.x - arrow_size, tip.y + arrow_size * 0.5);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, left, right],
                STATE_DEFAULT_COLOR,
                Stroke::NONE,
            ));
        }

        // ---- Interaction: pan, zoom, drag nodes ----

        // Zoom with scroll wheel
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && canvas_rect.contains(ui.input(|i| {
            i.pointer.hover_pos().unwrap_or(Pos2::ZERO)
        })) {
            let new_zoom = (self.editor_ctx.animator_graph_zoom + scroll_delta * 0.005)
                .clamp(MIN_ZOOM, MAX_ZOOM);
            self.editor_ctx.animator_graph_zoom = new_zoom;
        }

        // Pan with middle mouse or Alt+left drag
        let is_middle_drag = ui.input(|i| i.pointer.middle_down());
        let is_alt_drag = ui.input(|i| {
            i.modifiers.alt && i.pointer.primary_down()
        });

        if (is_middle_drag || is_alt_drag) && response.dragged() {
            self.editor_ctx.animator_graph_pan += response.drag_delta();
            self.editor_ctx.animator_graph_dragging = None; // cancel node drag during pan
        }

        // Node dragging (left click, no Alt)
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());

        if response.drag_started()
            && !is_middle_drag
            && !is_alt_drag
        {
            if let Some(pos) = pointer_pos {
                // Find which node was clicked
                for (i, rect) in node_rects.iter().enumerate() {
                    if rect.contains(pos) {
                        self.editor_ctx.animator_graph_dragging = Some(i);
                        break;
                    }
                }
            }
        }

        if let Some(dragging_idx) = self.editor_ctx.animator_graph_dragging {
            if response.dragged() && !is_middle_drag && !is_alt_drag {
                let delta = response.drag_delta();
                let current_pos = node_positions.get(dragging_idx).copied().unwrap_or(Pos2::ZERO);
                let inv_zoom = if zoom > 0.0 { 1.0 / zoom } else { 1.0 };
                let new_pos = (
                    current_pos.x + delta.x * inv_zoom,
                    current_pos.y + delta.y * inv_zoom,
                );
                self.editor_ctx
                    .animator_graph_positions
                    .insert(dragging_idx, new_pos);
            }
        }

        if response.drag_stopped() {
            self.editor_ctx.animator_graph_dragging = None;
        }

        // ---- Right-click context menu ----
        response.context_menu(|ui| {
            ui.set_min_width(140.0);
            ui.label(
                egui::RichText::new("Animator Graph")
                    .color(theme::TEXT_DISABLED)
                    .size(11.0),
            );
            ui.separator();
            if ui.button("Reset View").clicked() {
                self.editor_ctx.animator_graph_pan = Vec2::ZERO;
                self.editor_ctx.animator_graph_zoom = 1.0;
                ui.close();
            }
            if ui.button("Auto Layout").clicked() {
                self.editor_ctx.animator_graph_positions.clear();
                ui.close();
            }
            ui.separator();
            ui.label(
                egui::RichText::new("(Read-only view)")
                    .color(theme::TEXT_DISABLED)
                    .italics()
                    .size(10.0),
            );
        });

        // ---- Zoom indicator ----
        let zoom_text = format!("{:.0}%", zoom * 100.0);
        painter.text(
            Pos2::new(canvas_rect.max.x - 8.0, canvas_rect.max.y - 8.0),
            egui::Align2::RIGHT_BOTTOM,
            &zoom_text,
            egui::FontId::proportional(10.0),
            theme::TEXT_DISABLED,
        );

        // ---- Help text ----
        painter.text(
            Pos2::new(canvas_rect.min.x + 8.0, canvas_rect.max.y - 8.0),
            egui::Align2::LEFT_BOTTOM,
            "MMB: Pan  |  Scroll: Zoom  |  Drag: Move nodes  |  RMB: Menu",
            egui::FontId::proportional(9.0),
            theme::TEXT_DISABLED.gamma_multiply(0.6),
        );
    }
}

// ---- Drawing helpers ----

fn draw_grid(painter: &egui::Painter, rect: Rect, offset: Vec2, size: f32, color: Color32) {
    if size < 4.0 {
        return; // avoid drawing impossibly dense grid
    }
    let start_x = rect.min.x + (offset.x % size);
    let start_y = rect.min.y + (offset.y % size);

    let mut x = start_x;
    while x < rect.max.x {
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(0.5, color),
        );
        x += size;
    }

    let mut y = start_y;
    while y < rect.max.y {
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            Stroke::new(0.5, color),
        );
        y += size;
    }
}

fn draw_state_node(
    painter: &egui::Painter,
    state: &AnimationState,
    rect: Rect,
    is_current: bool,
    is_default: bool,
    is_blending: bool,
    zoom: f32,
) {
    let corner = CornerRadius::same((NODE_CORNER_RADIUS * zoom) as u8);

    // Glow effect for active state
    if is_current {
        let glow_rect = rect.expand(4.0 * zoom);
        let glow_color = STATE_ACTIVE_COLOR.gamma_multiply(0.2);
        painter.rect_filled(glow_rect, CornerRadius::same(((NODE_CORNER_RADIUS + 4.0) * zoom) as u8), glow_color);
    }

    // Background
    let bg = if is_current {
        STATE_ACTIVE_COLOR.gamma_multiply(0.15)
    } else {
        STATE_BG
    };
    painter.rect_filled(rect, corner, bg);

    // Border
    let border_color = if is_current {
        STATE_ACTIVE_COLOR
    } else if is_default {
        STATE_DEFAULT_COLOR
    } else {
        STATE_BORDER
    };
    let border_width = if is_current || is_default { 2.0 } else { 1.0 };
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(border_width * zoom, border_color),
        egui::StrokeKind::Outside,
    );

    // Blending indicator (pulsing border)
    if is_blending {
        let pulse_rect = rect.expand(2.0 * zoom);
        painter.rect_stroke(
            pulse_rect,
            CornerRadius::same(((NODE_CORNER_RADIUS + 2.0) * zoom) as u8),
            Stroke::new(1.0 * zoom, TRANSITION_COLOR.gamma_multiply(0.5)),
            egui::StrokeKind::Outside,
        );
    }

    // State name
    painter.text(
        rect.center_top() + Vec2::new(0.0, 16.0 * zoom),
        egui::Align2::CENTER_CENTER,
        &state.name,
        egui::FontId::proportional(13.0 * zoom),
        Color32::WHITE,
    );

    // Clip name (smaller, dimmer)
    if let Some(ref clip) = state.clip_name {
        painter.text(
            rect.center_top() + Vec2::new(0.0, 34.0 * zoom),
            egui::Align2::CENTER_CENTER,
            clip,
            egui::FontId::proportional(10.0 * zoom),
            CLIP_TEXT_COLOR,
        );
    } else {
        painter.text(
            rect.center_top() + Vec2::new(0.0, 34.0 * zoom),
            egui::Align2::CENTER_CENTER,
            "(no clip)",
            egui::FontId::proportional(10.0 * zoom),
            theme::TEXT_DISABLED,
        );
    }
}

fn draw_transition_arrow(
    painter: &egui::Painter,
    from_rect: Rect,
    to_rect: Rect,
    color: Color32,
    zoom: f32,
) {
    let from_center = from_rect.center();
    let to_center = to_rect.center();
    let diff = to_center - from_center;
    let len = diff.length();
    if len < 1.0 {
        return;
    }
    let dir = diff / len;

    // Compute exit/entry points on the rect edges
    let start = edge_intersection(from_rect, from_center, dir);
    let end = edge_intersection(to_rect, to_center, -dir);

    // Line
    painter.line_segment([start, end], Stroke::new(2.0 * zoom, color));

    // Arrowhead
    let arrow_size = 8.0 * zoom;
    let perp = Vec2::new(-dir.y, dir.x);
    let tip = end;
    let left = tip - dir * arrow_size + perp * arrow_size * 0.5;
    let right = tip - dir * arrow_size - perp * arrow_size * 0.5;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        Stroke::NONE,
    ));
}

/// Find where a ray from `center` in direction `dir` exits `rect`.
fn edge_intersection(rect: Rect, center: Pos2, dir: Vec2) -> Pos2 {
    // Check all four edges and return the closest intersection
    let mut best_t = f32::MAX;

    // Right edge (x = rect.max.x)
    if dir.x > 0.0 {
        let t = (rect.max.x - center.x) / dir.x;
        let y = center.y + t * dir.y;
        if y >= rect.min.y && y <= rect.max.y && t < best_t {
            best_t = t;
        }
    }
    // Left edge (x = rect.min.x)
    if dir.x < 0.0 {
        let t = (rect.min.x - center.x) / dir.x;
        let y = center.y + t * dir.y;
        if y >= rect.min.y && y <= rect.max.y && t < best_t {
            best_t = t;
        }
    }
    // Bottom edge (y = rect.max.y)
    if dir.y > 0.0 {
        let t = (rect.max.y - center.y) / dir.y;
        let x = center.x + t * dir.x;
        if x >= rect.min.x && x <= rect.max.x && t < best_t {
            best_t = t;
        }
    }
    // Top edge (y = rect.min.y)
    if dir.y < 0.0 {
        let t = (rect.min.y - center.y) / dir.y;
        let x = center.x + t * dir.x;
        if x >= rect.min.x && x <= rect.max.x && t < best_t {
            best_t = t;
        }
    }

    if best_t < f32::MAX {
        Pos2::new(center.x + best_t * dir.x, center.y + best_t * dir.y)
    } else {
        center
    }
}
