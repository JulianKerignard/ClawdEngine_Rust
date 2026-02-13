use std::path::Path;

use egui::{Color32, ColorImage, CornerRadius, Frame, Stroke, Vec2};
use image::Rgba;

use super::context::{EditorContext, HubAction};
use super::theme;

struct SceneEntry {
    name: String,
    preview_path: Option<String>,
}

fn discover_scenes() -> Vec<SceneEntry> {
    let scenes_dir = Path::new("assets/scenes");
    if !scenes_dir.exists() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    if let Ok(dir) = std::fs::read_dir(scenes_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ron") {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let preview = format!("assets/scenes/{}.png", name);
                let preview_path = if Path::new(&preview).exists() {
                    Some(preview)
                } else {
                    None
                };

                entries.push(SceneEntry { name, preview_path });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn load_thumbnail(
    ctx: &egui::Context,
    editor_ctx: &mut EditorContext,
    scene_name: &str,
    path: &str,
) -> Option<egui::TextureHandle> {
    // Check cache first
    if let Some(handle) = editor_ctx.hub_thumbnails.get(scene_name) {
        return Some(handle.clone());
    }

    // Load PNG from disk
    let img = image::open(path).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let pixels: Vec<Color32> = img
        .pixels()
        .map(|p: &Rgba<u8>| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_image = ColorImage {
        size,
        source_size: Vec2::new(size[0] as f32, size[1] as f32),
        pixels,
    };

    let handle = ctx.load_texture(
        format!("scene_preview_{}", scene_name),
        color_image,
        egui::TextureOptions::LINEAR,
    );

    editor_ctx
        .hub_thumbnails
        .insert(scene_name.to_string(), handle.clone());
    Some(handle)
}

pub fn show(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(theme::BG_CRUST))
        .show(ctx, |ui| {
            let available = ui.available_size();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((available.y * 0.10).max(24.0));

                    // Title
                    ui.label(
                        egui::RichText::new("ClawdEngine")
                            .color(theme::ACCENT)
                            .size(36.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("3D Game Engine")
                            .color(theme::TEXT_DISABLED)
                            .size(14.0),
                    );

                    ui.add_space(36.0);

                    // New project buttons
                    ui.horizontal(|ui| {
                        let button_width = 200.0;
                        let total_width = button_width * 2.0 + 16.0;
                        let offset = (ui.available_width() - total_width) / 2.0;
                        ui.add_space(offset.max(0.0));

                        if new_project_card(
                            ui,
                            "New Blank Scene",
                            "Empty world, start from scratch",
                            "+",
                            button_width,
                        ) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewBlank);
                        }

                        ui.add_space(16.0);

                        if new_project_card(
                            ui,
                            "Demo Scene",
                            "Lights, meshes, scripts & physics",
                            "\u{25B6}",
                            button_width,
                        ) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewDemo);
                        }
                    });

                    ui.add_space(36.0);

                    // Recent scenes
                    let scenes = discover_scenes();
                    if !scenes.is_empty() {
                        ui.label(
                            egui::RichText::new("Recent Scenes")
                                .color(theme::TEXT_SECONDARY)
                                .size(16.0)
                                .strong(),
                        );
                        ui.add_space(16.0);

                        // Grid of scene cards
                        let card_width: f32 = 200.0;
                        let spacing: f32 = 12.0;
                        let max_per_row = ((ui.available_width() + spacing)
                            / (card_width + spacing))
                            .floor()
                            .max(1.0) as usize;

                        let row_width =
                            max_per_row as f32 * (card_width + spacing) - spacing;
                        let grid_offset =
                            (ui.available_width() - row_width) / 2.0;
                        ui.add_space(grid_offset.max(0.0));

                        let mut clicked_scene: Option<String> = None;

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing =
                                egui::vec2(spacing, spacing);
                            for entry in &scenes {
                                let thumb = entry.preview_path.as_ref().and_then(|p| {
                                    load_thumbnail(ctx, editor_ctx, &entry.name, p)
                                });
                                if scene_card(ui, &entry.name, thumb.as_ref(), card_width)
                                {
                                    clicked_scene = Some(entry.name.clone());
                                }
                            }
                        });

                        if let Some(name) = clicked_scene {
                            editor_ctx.pending_hub_action =
                                Some(HubAction::OpenScene(name));
                        }
                    } else {
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new("No saved scenes yet")
                                .color(theme::TEXT_DISABLED)
                                .size(13.0),
                        );
                    }

                    ui.add_space(48.0);

                    // Version
                    ui.label(
                        egui::RichText::new("v0.1.0")
                            .color(theme::TEXT_DISABLED)
                            .size(11.0),
                    );

                    ui.add_space(16.0);
                });
            });
        });
}

fn new_project_card(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    icon: &str,
    width: f32,
) -> bool {
    let height = 80.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let hovered = response.hovered();
    let bg = if hovered {
        theme::BG_SURFACE0
    } else {
        theme::BG_MANTLE
    };
    let stroke = if hovered {
        Stroke::new(1.5, theme::ACCENT)
    } else {
        Stroke::new(1.0, theme::BG_SURFACE0)
    };

    ui.painter()
        .rect(rect, CornerRadius::same(8), bg, stroke, egui::StrokeKind::Outside);

    // Icon
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(22.0),
        theme::ACCENT,
    );

    // Title
    ui.painter().text(
        egui::pos2(rect.left() + 52.0, rect.center().y - 10.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        theme::TEXT_PRIMARY,
    );

    // Description
    ui.painter().text(
        egui::pos2(rect.left() + 52.0, rect.center().y + 10.0),
        egui::Align2::LEFT_CENTER,
        description,
        egui::FontId::proportional(11.0),
        theme::TEXT_DISABLED,
    );

    response.clicked()
}

fn scene_card(
    ui: &mut egui::Ui,
    name: &str,
    thumbnail: Option<&egui::TextureHandle>,
    width: f32,
) -> bool {
    let thumb_height = (width * 0.5625).round(); // 16:9
    let total_height = thumb_height + 36.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, total_height), egui::Sense::click());

    let hovered = response.hovered();
    let bg = if hovered {
        theme::BG_SURFACE0
    } else {
        theme::BG_MANTLE
    };
    let stroke = if hovered {
        Stroke::new(1.5, theme::ACCENT)
    } else {
        Stroke::new(1.0, theme::BG_SURFACE0)
    };

    // Card background
    ui.painter()
        .rect(rect, CornerRadius::same(8), bg, stroke, egui::StrokeKind::Outside);

    // Thumbnail zone
    let thumb_rect =
        egui::Rect::from_min_size(rect.min, egui::vec2(width, thumb_height));

    if let Some(handle) = thumbnail {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let inner = thumb_rect.shrink(1.0);
        // Clip top corners
        ui.painter().rect_filled(
            egui::Rect::from_min_size(inner.min, egui::vec2(inner.width(), inner.height())),
            CornerRadius {
                nw: 7,
                ne: 7,
                sw: 0,
                se: 0,
            },
            theme::BG_BASE,
        );
        ui.painter()
            .image(handle.id(), inner, uv, Color32::WHITE);
    } else {
        // Placeholder
        ui.painter().rect_filled(
            thumb_rect.shrink(1.0),
            CornerRadius {
                nw: 7,
                ne: 7,
                sw: 0,
                se: 0,
            },
            theme::BG_BASE,
        );
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{1F3A8}",
            egui::FontId::proportional(28.0),
            theme::TEXT_DISABLED,
        );
    }

    // Scene name
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, thumb_rect.bottom() + 10.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(12.0),
        if hovered {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        },
    );

    response.clicked()
}
