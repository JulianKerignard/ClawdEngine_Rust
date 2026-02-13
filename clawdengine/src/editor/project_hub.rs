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
    if let Some(handle) = editor_ctx.hub_thumbnails.get(scene_name) {
        return Some(handle.clone());
    }

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

fn delete_scene(name: &str, editor_ctx: &mut EditorContext) {
    let ron_path = format!("assets/scenes/{}.ron", name);
    let png_path = format!("assets/scenes/{}.png", name);
    let _ = std::fs::remove_file(&ron_path);
    let _ = std::fs::remove_file(&png_path);
    editor_ctx.hub_thumbnails.remove(name);
    log::info!("Deleted scene: {}", name);
}

fn duplicate_scene(name: &str) {
    let ron_src = format!("assets/scenes/{}.ron", name);
    let png_src = format!("assets/scenes/{}.png", name);

    // Find unique name
    let mut copy_name = format!("{} Copy", name);
    let mut i = 2;
    while Path::new(&format!("assets/scenes/{}.ron", copy_name)).exists() {
        copy_name = format!("{} Copy {}", name, i);
        i += 1;
    }

    let ron_dst = format!("assets/scenes/{}.ron", copy_name);
    let png_dst = format!("assets/scenes/{}.png", copy_name);
    let _ = std::fs::copy(&ron_src, &ron_dst);
    if Path::new(&png_src).exists() {
        let _ = std::fs::copy(&png_src, &png_dst);
    }
    log::info!("Duplicated scene '{}' → '{}'", name, copy_name);
}

fn rename_scene(old_name: &str, new_name: &str, editor_ctx: &mut EditorContext) {
    if old_name == new_name || new_name.is_empty() {
        return;
    }
    let old_ron = format!("assets/scenes/{}.ron", old_name);
    let new_ron = format!("assets/scenes/{}.ron", new_name);
    let old_png = format!("assets/scenes/{}.png", old_name);
    let new_png = format!("assets/scenes/{}.png", new_name);

    if Path::new(&new_ron).exists() {
        log::warn!("Cannot rename: '{}' already exists", new_name);
        return;
    }

    let _ = std::fs::rename(&old_ron, &new_ron);
    if Path::new(&old_png).exists() {
        let _ = std::fs::rename(&old_png, &new_png);
    }
    // Update thumbnail cache
    if let Some(handle) = editor_ctx.hub_thumbnails.remove(old_name) {
        editor_ctx.hub_thumbnails.insert(new_name.to_string(), handle);
    }
    log::info!("Renamed scene '{}' → '{}'", old_name, new_name);
}

pub fn show(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(theme::BG_CRUST))
        .show(ctx, |ui| {
            let available = ui.available_size();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((available.y * 0.08).max(20.0));

                    // Title
                    ui.label(
                        egui::RichText::new("ClawdEngine")
                            .color(theme::ACCENT)
                            .size(32.0)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("3D Game Engine")
                            .color(theme::TEXT_DISABLED)
                            .size(12.0),
                    );

                    ui.add_space(28.0);

                    // New project buttons
                    ui.horizontal(|ui| {
                        let btn_w = 180.0;
                        let total = btn_w * 2.0 + 12.0;
                        let offset = (ui.available_width() - total) / 2.0;
                        ui.add_space(offset.max(0.0));

                        if new_project_card(ui, "New Blank Scene", "Empty world", "+", btn_w) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewBlank);
                        }

                        ui.add_space(12.0);

                        if new_project_card(
                            ui,
                            "Demo Scene",
                            "Lights, meshes & scripts",
                            "\u{25B6}",
                            btn_w,
                        ) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewDemo);
                        }
                    });

                    ui.add_space(32.0);

                    // Recent scenes
                    let scenes = discover_scenes();
                    if !scenes.is_empty() {
                        ui.label(
                            egui::RichText::new("Recent Scenes")
                                .color(theme::TEXT_SECONDARY)
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(12.0);

                        let card_w: f32 = 150.0;
                        let spacing: f32 = 10.0;

                        let mut clicked_scene: Option<String> = None;
                        let mut delete_scene_name: Option<String> = None;
                        let mut duplicate_scene_name: Option<String> = None;
                        let mut rename_scene_name: Option<String> = None;

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
                            for entry in &scenes {
                                let thumb = entry.preview_path.as_ref().and_then(|p| {
                                    load_thumbnail(ctx, editor_ctx, &entry.name, p)
                                });
                                let result =
                                    scene_card_with_menu(ui, &entry.name, thumb.as_ref(), card_w);
                                match result {
                                    CardAction::Open => {
                                        clicked_scene = Some(entry.name.clone());
                                    }
                                    CardAction::Delete => {
                                        delete_scene_name = Some(entry.name.clone());
                                    }
                                    CardAction::Duplicate => {
                                        duplicate_scene_name = Some(entry.name.clone());
                                    }
                                    CardAction::Rename => {
                                        rename_scene_name = Some(entry.name.clone());
                                    }
                                    CardAction::None => {}
                                }
                            }
                        });

                        if let Some(name) = clicked_scene {
                            editor_ctx.pending_hub_action = Some(HubAction::OpenScene(name));
                        }
                        if let Some(name) = delete_scene_name {
                            delete_scene(&name, editor_ctx);
                        }
                        if let Some(name) = duplicate_scene_name {
                            duplicate_scene(&name);
                        }
                        if let Some(name) = rename_scene_name {
                            editor_ctx.hub_rename = Some((name.clone(), name));
                        }
                    } else {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("No saved scenes yet")
                                .color(theme::TEXT_DISABLED)
                                .size(12.0),
                        );
                    }

                    ui.add_space(40.0);

                    ui.label(
                        egui::RichText::new("v0.1.0")
                            .color(theme::TEXT_DISABLED)
                            .size(10.0),
                    );

                    ui.add_space(12.0);
                });
            });
        });

    // Rename modal (rendered on top)
    show_rename_modal(ctx, editor_ctx);
}

fn show_rename_modal(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    if editor_ctx.hub_rename.is_none() {
        return;
    }

    let mut do_rename = false;
    let mut do_cancel = false;

    egui::Window::new("Rename Scene")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some((ref original, ref mut new_name)) = editor_ctx.hub_rename {
                ui.label(
                    egui::RichText::new(format!("Rename \"{}\"", original))
                        .color(theme::TEXT_SECONDARY)
                        .size(12.0),
                );
                ui.add_space(8.0);

                let response = ui.text_edit_singleline(new_name);
                if response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    do_rename = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });
            }
        });

    if do_rename {
        if let Some((old, new)) = editor_ctx.hub_rename.take() {
            rename_scene(&old, &new, editor_ctx);
        }
    } else if do_cancel {
        editor_ctx.hub_rename = None;
    }
}

enum CardAction {
    None,
    Open,
    Delete,
    Duplicate,
    Rename,
}

fn new_project_card(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    icon: &str,
    width: f32,
) -> bool {
    let height = 56.0;
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
        .rect(rect, CornerRadius::same(6), bg, stroke, egui::StrokeKind::Outside);

    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(18.0),
        theme::ACCENT,
    );

    ui.painter().text(
        egui::pos2(rect.left() + 38.0, rect.center().y - 8.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        theme::TEXT_PRIMARY,
    );

    ui.painter().text(
        egui::pos2(rect.left() + 38.0, rect.center().y + 9.0),
        egui::Align2::LEFT_CENTER,
        description,
        egui::FontId::proportional(10.0),
        theme::TEXT_DISABLED,
    );

    response.clicked()
}

fn scene_card_with_menu(
    ui: &mut egui::Ui,
    name: &str,
    thumbnail: Option<&egui::TextureHandle>,
    width: f32,
) -> CardAction {
    let thumb_height = (width * 0.5625).round();
    let total_height = thumb_height + 28.0;

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

    ui.painter()
        .rect(rect, CornerRadius::same(6), bg, stroke, egui::StrokeKind::Outside);

    // Thumbnail
    let thumb_rect = egui::Rect::from_min_size(rect.min, egui::vec2(width, thumb_height));

    if let Some(handle) = thumbnail {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let inner = thumb_rect.shrink(1.0);
        ui.painter().rect_filled(
            inner,
            CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
            theme::BG_BASE,
        );
        ui.painter().image(handle.id(), inner, uv, Color32::WHITE);
    } else {
        ui.painter().rect_filled(
            thumb_rect.shrink(1.0),
            CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
            theme::BG_BASE,
        );
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{1F3AE}",
            egui::FontId::proportional(22.0),
            theme::TEXT_DISABLED,
        );
    }

    // Scene name
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, thumb_rect.bottom() + 7.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(11.0),
        if hovered { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
    );

    // Context menu (right-click)
    let mut action = CardAction::None;
    response.context_menu(|ui| {
        if ui.button("\u{1F4C2}  Open").clicked() {
            action = CardAction::Open;
            ui.close();
        }
        ui.separator();
        if ui.button("\u{270F}  Rename").clicked() {
            action = CardAction::Rename;
            ui.close();
        }
        if ui.button("\u{1F4CB}  Duplicate").clicked() {
            action = CardAction::Duplicate;
            ui.close();
        }
        ui.separator();
        if ui
            .add(egui::Button::new(
                egui::RichText::new("\u{1F5D1}  Delete").color(theme::ERROR),
            ))
            .clicked()
        {
            action = CardAction::Delete;
            ui.close();
        }
    });

    // Left-click opens (only if context menu didn't trigger)
    if matches!(action, CardAction::None) && response.clicked() {
        action = CardAction::Open;
    }

    action
}
