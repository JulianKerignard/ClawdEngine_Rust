use std::path::Path;

use egui::{Color32, ColorImage, CornerRadius, Frame, Stroke, Vec2};
use image::Rgba;

use crate::assets::project::{self, ProjectManifest, ProjectRegistry};
use super::context::{EditorContext, HubAction, HubSortMode};
use super::theme;

struct ProjectEntry {
    path: String,
    manifest: Option<ProjectManifest>,
    scene_count: usize,
    entity_count: usize,
    last_opened: String,
    exists: bool,
}

fn discover_projects(registry: &ProjectRegistry) -> Vec<ProjectEntry> {
    registry
        .recent
        .iter()
        .map(|rp| {
            let dir = Path::new(&rp.path);
            let exists = dir.exists() && dir.join("project.ron").exists();
            let manifest = if exists {
                project::load_manifest(&rp.path).ok()
            } else {
                None
            };
            let scene_count = if exists {
                project::count_scenes_in_project(&rp.path)
            } else {
                0
            };
            let entity_count = if exists {
                manifest
                    .as_ref()
                    .and_then(|m| m.default_scene.as_ref())
                    .and_then(|s| {
                        let scene_path =
                            format!("{}/scenes/{}.ron", rp.path, s);
                        project::count_entities_in_scene(&scene_path)
                    })
                    .unwrap_or(0)
            } else {
                0
            };
            ProjectEntry {
                path: rp.path.clone(),
                manifest,
                scene_count,
                entity_count,
                last_opened: rp.last_opened.clone(),
                exists,
            }
        })
        .collect()
}

fn sort_projects(entries: &mut [ProjectEntry], mode: HubSortMode) {
    match mode {
        HubSortMode::NameAsc => entries.sort_by(|a, b| {
            let na = a.manifest.as_ref().map(|m| &m.name).unwrap_or(&a.path);
            let nb = b.manifest.as_ref().map(|m| &m.name).unwrap_or(&b.path);
            na.to_lowercase().cmp(&nb.to_lowercase())
        }),
        HubSortMode::NameDesc => entries.sort_by(|a, b| {
            let na = a.manifest.as_ref().map(|m| &m.name).unwrap_or(&a.path);
            let nb = b.manifest.as_ref().map(|m| &m.name).unwrap_or(&b.path);
            nb.to_lowercase().cmp(&na.to_lowercase())
        }),
        HubSortMode::DateDesc => {
            entries.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        }
        HubSortMode::DateAsc => {
            entries.sort_by(|a, b| a.last_opened.cmp(&b.last_opened));
        }
    }
}

fn load_thumbnail(
    ctx: &egui::Context,
    editor_ctx: &mut EditorContext,
    key: &str,
    path: &str,
) -> Option<egui::TextureHandle> {
    if let Some(handle) = editor_ctx.hub_thumbnails.get(key) {
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
        format!("project_preview_{}", key),
        color_image,
        egui::TextureOptions::LINEAR,
    );

    editor_ctx
        .hub_thumbnails
        .insert(key.to_string(), handle.clone());
    Some(handle)
}

fn rename_project(old_path: &str, new_name: &str, editor_ctx: &mut EditorContext) {
    if new_name.is_empty() {
        return;
    }
    if let Ok(mut manifest) = project::load_manifest(old_path) {
        manifest.name = new_name.to_string();
        let _ = project::save_manifest(old_path, &manifest);
        editor_ctx.hub_thumbnails.remove(old_path);
        log::info!("Renamed project to '{}'", new_name);
    }
}

pub fn show(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    let registry = project::load_registry();
    let mut entries = discover_projects(&registry);
    sort_projects(&mut entries, editor_ctx.hub_sort_mode);

    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(theme::BG_CRUST))
        .show(ctx, |ui| {
            let available = ui.available_size();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((available.y * 0.06).max(16.0));

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

                    ui.add_space(24.0);

                    // Action bar: [New Blank] [Demo] [Import] | Sort
                    ui.horizontal(|ui| {
                        let btn_w = 150.0;
                        let total = btn_w * 3.0 + 24.0 + 120.0;
                        let offset = (ui.available_width() - total) / 2.0;
                        ui.add_space(offset.max(0.0));

                        if action_button(ui, "+  New Blank", btn_w) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewBlank);
                        }
                        ui.add_space(8.0);
                        if action_button(ui, "\u{25B6}  Demo Scene", btn_w) {
                            editor_ctx.pending_hub_action = Some(HubAction::NewDemo);
                        }
                        ui.add_space(8.0);
                        if action_button(ui, "\u{1F4C2}  Import", btn_w) {
                            if let Some(folder) = rfd::FileDialog::new()
                                .set_title("Open Project Folder")
                                .pick_folder()
                            {
                                let path_str = folder.to_string_lossy().to_string();
                                if folder.join("project.ron").exists() {
                                    editor_ctx.pending_hub_action =
                                        Some(HubAction::OpenProject(path_str));
                                } else {
                                    log::warn!(
                                        "Not a ClawdEngine project (no project.ron): {}",
                                        path_str
                                    );
                                }
                            }
                        }

                        ui.add_space(16.0);

                        // Sort combo
                        ui.label(
                            egui::RichText::new("Sort:")
                                .color(theme::TEXT_DISABLED)
                                .size(11.0),
                        );
                        let sort_label = match editor_ctx.hub_sort_mode {
                            HubSortMode::NameAsc => "Name A-Z",
                            HubSortMode::NameDesc => "Name Z-A",
                            HubSortMode::DateDesc => "Recent",
                            HubSortMode::DateAsc => "Oldest",
                        };
                        egui::ComboBox::from_id_salt("hub_sort")
                            .selected_text(
                                egui::RichText::new(sort_label)
                                    .color(theme::TEXT_SECONDARY)
                                    .size(11.0),
                            )
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut editor_ctx.hub_sort_mode,
                                    HubSortMode::DateDesc,
                                    "Recent",
                                );
                                ui.selectable_value(
                                    &mut editor_ctx.hub_sort_mode,
                                    HubSortMode::DateAsc,
                                    "Oldest",
                                );
                                ui.selectable_value(
                                    &mut editor_ctx.hub_sort_mode,
                                    HubSortMode::NameAsc,
                                    "Name A-Z",
                                );
                                ui.selectable_value(
                                    &mut editor_ctx.hub_sort_mode,
                                    HubSortMode::NameDesc,
                                    "Name Z-A",
                                );
                            });
                    });

                    ui.add_space(28.0);

                    // Projects grid
                    if !entries.is_empty() {
                        ui.label(
                            egui::RichText::new("Projects")
                                .color(theme::TEXT_SECONDARY)
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(12.0);

                        let card_w: f32 = 170.0;
                        let spacing: f32 = 10.0;

                        let mut open_path: Option<String> = None;
                        let mut delete_path: Option<String> = None;
                        let mut rename_path: Option<String> = None;
                        let mut reveal_path: Option<String> = None;

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
                            for entry in &entries {
                                let thumb_path =
                                    format!("{}/preview.png", entry.path);
                                let thumb = if entry.exists {
                                    load_thumbnail(
                                        ctx,
                                        editor_ctx,
                                        &entry.path,
                                        &thumb_path,
                                    )
                                } else {
                                    None
                                };

                                let name = entry
                                    .manifest
                                    .as_ref()
                                    .map(|m| m.name.as_str())
                                    .unwrap_or("???");
                                let date_str =
                                    project::format_relative_date(&entry.last_opened);

                                let result = project_card(
                                    ui,
                                    name,
                                    entry.scene_count,
                                    entry.entity_count,
                                    &date_str,
                                    entry.exists,
                                    thumb.as_ref(),
                                    card_w,
                                );

                                match result {
                                    CardAction::Open => {
                                        open_path = Some(entry.path.clone());
                                    }
                                    CardAction::Delete => {
                                        delete_path = Some(entry.path.clone());
                                    }
                                    CardAction::Rename => {
                                        rename_path = Some(entry.path.clone());
                                    }
                                    CardAction::Reveal => {
                                        reveal_path = Some(entry.path.clone());
                                    }
                                    CardAction::None => {}
                                }
                            }
                        });

                        if let Some(path) = open_path {
                            editor_ctx.pending_hub_action =
                                Some(HubAction::OpenProject(path));
                        }
                        if let Some(path) = delete_path {
                            editor_ctx.hub_delete_confirm = Some(path);
                        }
                        if let Some(path) = rename_path {
                            let name = entries
                                .iter()
                                .find(|e| e.path == path)
                                .and_then(|e| e.manifest.as_ref())
                                .map(|m| m.name.clone())
                                .unwrap_or_default();
                            editor_ctx.hub_rename = Some((path, name));
                        }
                        if let Some(path) = reveal_path {
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg(&path)
                                    .spawn();
                            }
                        }
                    } else {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new(
                                "No projects yet \u{2014} create one or import!",
                            )
                            .color(theme::TEXT_DISABLED)
                            .size(13.0),
                        );
                    }

                    // Legacy scenes section (from assets/scenes/ in original CWD)
                    let legacy_dir = format!("{}/assets/scenes", editor_ctx.original_cwd);
                    let legacy_scenes = discover_legacy_scenes(&legacy_dir);
                    if !legacy_scenes.is_empty() {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("Legacy Scenes (not in a project)")
                                .color(theme::TEXT_DISABLED)
                                .size(12.0),
                        );
                        ui.add_space(8.0);

                        let mut migrate_scene: Option<(String, String)> = None;

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                            for ls in &legacy_scenes {
                                let thumb = ls.preview_path.as_ref().and_then(|p| {
                                    load_thumbnail(ctx, editor_ctx, &format!("legacy_{}", ls.name), p)
                                });
                                if legacy_scene_card(ui, &ls.name, thumb.as_ref(), 140.0) {
                                    migrate_scene = Some((ls.name.clone(), legacy_dir.clone()));
                                }
                            }
                        });

                        if let Some((name, dir)) = migrate_scene {
                            migrate_legacy_to_project(&name, &dir, editor_ctx);
                        }
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

    // Modals (rendered on top)
    show_rename_modal(ctx, editor_ctx);
    show_delete_confirm_modal(ctx, editor_ctx);
}

// ---- Delete Confirmation Modal ----

fn show_delete_confirm_modal(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    if editor_ctx.hub_delete_confirm.is_none() {
        return;
    }

    let mut do_delete = false;
    let mut do_cancel = false;

    egui::Window::new("Remove Project")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(ref path) = editor_ctx.hub_delete_confirm {
                let name = project::load_manifest(path)
                    .map(|m| m.name)
                    .unwrap_or_else(|_| path.clone());

                ui.label(
                    egui::RichText::new(format!("Remove \"{}\" from the list?", name))
                        .color(theme::TEXT_PRIMARY)
                        .size(13.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("The project files will NOT be deleted from disk.")
                        .color(theme::TEXT_DISABLED)
                        .size(11.0),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Remove").color(Color32::WHITE),
                        ).fill(theme::ERROR))
                        .clicked()
                    {
                        do_delete = true;
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });
            }
        });

    if do_delete {
        if let Some(path) = editor_ctx.hub_delete_confirm.take() {
            let mut registry = project::load_registry();
            project::unregister_project(&mut registry, &path);
            let _ = project::save_registry(&registry);
            editor_ctx.hub_thumbnails.remove(&path);
            log::info!("Removed project from hub: {}", path);
        }
    } else if do_cancel {
        editor_ctx.hub_delete_confirm = None;
    }
}

// ---- Rename Modal ----

fn show_rename_modal(ctx: &egui::Context, editor_ctx: &mut EditorContext) {
    if editor_ctx.hub_rename.is_none() {
        return;
    }

    let mut do_rename = false;
    let mut do_cancel = false;

    egui::Window::new("Rename Project")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some((ref _path, ref mut new_name)) = editor_ctx.hub_rename {
                ui.label(
                    egui::RichText::new("New project name:")
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
        if let Some((path, new_name)) = editor_ctx.hub_rename.take() {
            rename_project(&path, &new_name, editor_ctx);
        }
    } else if do_cancel {
        editor_ctx.hub_rename = None;
    }
}

// ---- Card Actions ----

enum CardAction {
    None,
    Open,
    Delete,
    Rename,
    Reveal,
}

// ---- Action Button (top bar) ----

fn action_button(ui: &mut egui::Ui, label: &str, width: f32) -> bool {
    let height = 32.0;
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
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        if hovered { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
    );

    response.clicked()
}

// ---- Project Card ----

fn project_card(
    ui: &mut egui::Ui,
    name: &str,
    scene_count: usize,
    entity_count: usize,
    date_str: &str,
    exists: bool,
    thumbnail: Option<&egui::TextureHandle>,
    width: f32,
) -> CardAction {
    let thumb_height = (width * 0.5).round();
    let info_height = 52.0;
    let total_height = thumb_height + info_height;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, total_height), egui::Sense::click());

    let hovered = response.hovered();
    let alpha = if exists { 1.0 } else { 0.4 };
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

    // Thumbnail area
    let thumb_rect = egui::Rect::from_min_size(rect.min, egui::vec2(width, thumb_height));

    if let Some(handle) = thumbnail {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let inner = thumb_rect.shrink(1.0);
        ui.painter().rect_filled(
            inner,
            CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
            theme::BG_BASE,
        );
        let tint = Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * alpha) as u8);
        ui.painter().image(handle.id(), inner, uv, tint);
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

    // Info area
    let info_top = thumb_rect.bottom() + 4.0;
    let text_x = rect.left() + 8.0;

    // Project name
    let name_color = if !exists {
        theme::TEXT_DISABLED
    } else if hovered {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    let display_name = if exists {
        name.to_string()
    } else {
        format!("{} (missing)", name)
    };
    ui.painter().text(
        egui::pos2(text_x, info_top),
        egui::Align2::LEFT_TOP,
        &display_name,
        egui::FontId::proportional(11.0),
        name_color,
    );

    // Metadata line 1: scenes + entities
    let meta1 = format!(
        "{} scene{} \u{2022} {} entities",
        scene_count,
        if scene_count != 1 { "s" } else { "" },
        entity_count,
    );
    ui.painter().text(
        egui::pos2(text_x, info_top + 15.0),
        egui::Align2::LEFT_TOP,
        &meta1,
        egui::FontId::proportional(9.0),
        theme::TEXT_DISABLED,
    );

    // Metadata line 2: date
    ui.painter().text(
        egui::pos2(text_x, info_top + 27.0),
        egui::Align2::LEFT_TOP,
        date_str,
        egui::FontId::proportional(9.0),
        theme::TEXT_DISABLED,
    );

    // Context menu
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
        if ui.button("\u{1F4C1}  Show in Finder").clicked() {
            action = CardAction::Reveal;
            ui.close();
        }
        ui.separator();
        if ui
            .add(egui::Button::new(
                egui::RichText::new("\u{1F5D1}  Remove").color(theme::ERROR),
            ))
            .clicked()
        {
            action = CardAction::Delete;
            ui.close();
        }
    });

    // Left-click opens
    if matches!(action, CardAction::None) && response.clicked() && exists {
        action = CardAction::Open;
    }

    action
}

// ---- Legacy Scene Support ----

struct LegacyScene {
    name: String,
    preview_path: Option<String>,
}

fn discover_legacy_scenes(scenes_dir: &str) -> Vec<LegacyScene> {
    let dir = Path::new(scenes_dir);
    if !dir.exists() {
        return Vec::new();
    }
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ron") {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let preview = format!("{}/{}.png", scenes_dir, name);
                let preview_path = if Path::new(&preview).exists() {
                    Some(preview)
                } else {
                    None
                };
                entries.push(LegacyScene { name, preview_path });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn legacy_scene_card(
    ui: &mut egui::Ui,
    name: &str,
    thumbnail: Option<&egui::TextureHandle>,
    width: f32,
) -> bool {
    let thumb_height = (width * 0.5).round();
    let total_height = thumb_height + 32.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, total_height), egui::Sense::click());

    let hovered = response.hovered();
    let bg = if hovered { theme::BG_SURFACE0 } else { theme::BG_MANTLE };
    let stroke = if hovered {
        Stroke::new(1.5, theme::ACCENT)
    } else {
        Stroke::new(1.0, theme::BG_SURFACE0)
    };

    ui.painter()
        .rect(rect, CornerRadius::same(6), bg, stroke, egui::StrokeKind::Outside);

    let thumb_rect = egui::Rect::from_min_size(rect.min, egui::vec2(width, thumb_height));
    if let Some(handle) = thumbnail {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let inner = thumb_rect.shrink(1.0);
        ui.painter().rect_filled(inner, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, theme::BG_BASE);
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
            egui::FontId::proportional(18.0),
            theme::TEXT_DISABLED,
        );
    }

    // Name + "click to convert" hint
    ui.painter().text(
        egui::pos2(rect.left() + 6.0, thumb_rect.bottom() + 4.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(10.0),
        if hovered { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
    );
    ui.painter().text(
        egui::pos2(rect.left() + 6.0, thumb_rect.bottom() + 17.0),
        egui::Align2::LEFT_TOP,
        "Click to convert to project",
        egui::FontId::proportional(8.0),
        theme::TEXT_DISABLED,
    );

    response.clicked()
}

fn migrate_legacy_to_project(scene_name: &str, legacy_dir: &str, editor_ctx: &mut EditorContext) {
    let projects_dir = project::default_projects_dir();
    let _ = std::fs::create_dir_all(&projects_dir);
    let parent = projects_dir.to_string_lossy().to_string();

    match project::create_project(&parent, scene_name) {
        Ok(project_path) => {
            // Copy the .ron scene file into the new project's scenes/
            let src_ron = format!("{}/{}.ron", legacy_dir, scene_name);
            let dst_ron = format!("{}/scenes/{}.ron", project_path, scene_name);
            let _ = std::fs::copy(&src_ron, &dst_ron);

            // Copy preview as project preview
            let src_png = format!("{}/{}.png", legacy_dir, scene_name);
            if Path::new(&src_png).exists() {
                let dst_preview = format!("{}/preview.png", project_path);
                let dst_scene_png = format!("{}/scenes/{}.png", project_path, scene_name);
                let _ = std::fs::copy(&src_png, &dst_preview);
                let _ = std::fs::copy(&src_png, &dst_scene_png);
            }

            // Copy original assets (textures, audio, meshes) for the scene to work
            let orig_assets = Path::new(&editor_ctx.original_cwd).join("assets");
            for subdir in &["textures", "audio", "meshes"] {
                let src_dir = orig_assets.join(subdir);
                if src_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&src_dir) {
                        for entry in entries.flatten() {
                            let src = entry.path();
                            if src.is_file() {
                                let dst = Path::new(&project_path).join(subdir).join(entry.file_name());
                                let _ = std::fs::copy(&src, &dst);
                            }
                        }
                    }
                }
            }

            // Update manifest with default scene
            if let Ok(mut manifest) = project::load_manifest(&project_path) {
                manifest.default_scene = Some(scene_name.to_string());
                let _ = project::save_manifest(&project_path, &manifest);
            }

            // Register and open
            let mut registry = project::load_registry();
            project::register_project(&mut registry, &project_path);
            let _ = project::save_registry(&registry);

            editor_ctx.pending_hub_action = Some(HubAction::OpenProject(project_path));
            log::info!("Migrated legacy scene '{}' to project", scene_name);
        }
        Err(e) => {
            log::error!("Failed to migrate scene '{}': {}", scene_name, e);
        }
    }
}
