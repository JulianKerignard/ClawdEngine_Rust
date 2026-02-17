use std::path::Path;

use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

use crate::assets::extracted_assets::ExtractedMaterial;
use crate::core::{AnimationClip, Skeleton};
use crate::editor::layout::{EditorTabViewer, component_section, property_row};
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    /// Show asset inspector when a file is selected in the Project Browser.
    pub(crate) fn show_asset_inspector(&mut self, ui: &mut egui::Ui) {
        if self.editor_ctx.selected_assets.is_empty() {
            return;
        }

        // Multi-selection summary
        if self.editor_ctx.selected_assets.len() > 1 {
            self.show_multi_asset_inspector(ui);
            return;
        }

        let asset_path = self.editor_ctx.selected_assets[0].clone();

        let file_name = asset_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = file_name.as_str();

        // Header
        ui.horizontal(|ui| {
            ui.strong("Inspector");
            ui.label(
                egui::RichText::new(" Asset")
                    .small()
                    .strong()
                    .color(theme::ACCENT)
                    .background_color(theme::ACCENT.gamma_multiply(0.15)),
            );
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Asset header card
            self.asset_header(ui, &file_name, &asset_path);
            ui.add_space(6.0);

            // Type-specific inspector
            if extension.ends_with(".mat.ron") {
                self.inspect_material(ui, &asset_path);
            } else if extension.ends_with(".skel.ron") {
                self.inspect_skeleton(ui, &asset_path);
            } else if extension.ends_with(".clip.ron") {
                self.inspect_clip(ui, &asset_path);
            } else if extension.ends_with(".png")
                || extension.ends_with(".jpg")
                || extension.ends_with(".jpeg")
                || extension.ends_with(".bmp")
                || extension.ends_with(".tga")
            {
                self.inspect_image(ui, &asset_path);
            } else if extension.ends_with(".fbx")
                || extension.ends_with(".glb")
                || extension.ends_with(".gltf")
                || extension.ends_with(".obj")
            {
                self.inspect_mesh_file(ui, &asset_path);
            } else if extension.ends_with(".scene.ron") {
                self.inspect_scene_file(ui, &asset_path);
            } else if extension.ends_with(".rs") {
                self.inspect_script_file(ui, &asset_path);
            } else if extension.ends_with(".wav")
                || extension.ends_with(".mp3")
                || extension.ends_with(".ogg")
            {
                self.inspect_audio_file(ui, &asset_path);
            } else {
                self.inspect_generic_file(ui, &asset_path);
            }
        });
    }

    fn asset_header(&self, ui: &mut egui::Ui, file_name: &str, asset_path: &Path) {
        let (icon, color) = asset_type_icon(file_name);

        Frame::NONE
            .fill(theme::BG_MANTLE)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).color(color).size(18.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(file_name)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(asset_path.to_string_lossy().as_ref())
                                .color(theme::TEXT_DISABLED)
                                .size(10.0),
                        );
                    });
                });

                // File size
                if let Ok(meta) = std::fs::metadata(asset_path) {
                    let size = meta.len();
                    let display = if size > 1_048_576 {
                        format!("{:.1} MB", size as f64 / 1_048_576.0)
                    } else if size > 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{} B", size)
                    };
                    ui.label(
                        egui::RichText::new(format!("Size: {}", display))
                            .color(theme::TEXT_DISABLED)
                            .size(10.0),
                    );
                }
            });
    }

    // ---- Material (.mat.ron) ----
    fn inspect_material(&mut self, ui: &mut egui::Ui, path: &Path) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Failed to read: {}", e));
                return;
            }
        };
        let mut mat: ExtractedMaterial = match ron::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Parse error: {}", e));
                return;
            }
        };

        // --- Sphere preview (centered, large) ---
        let sphere_size = ui.available_width().min(128.0);
        let sphere_key = format!("insp_mat_{}", path.to_string_lossy());
        let sphere_id = if let Some(handle) = self.editor_ctx.asset_thumbnails.get(&sphere_key) {
            Some(handle.id())
        } else {
            let sphere_img = crate::editor::icons::generate_material_sphere(
                mat.albedo[0] * 255.0,
                mat.albedo[1] * 255.0,
                mat.albedo[2] * 255.0,
            );
            let size = [sphere_img.width() as usize, sphere_img.height() as usize];
            let pixels: Vec<egui::Color32> = sphere_img.pixels()
                .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();
            let color_img = egui::ColorImage { size, source_size: egui::Vec2::new(size[0] as f32, size[1] as f32), pixels };
            let handle = ui.ctx().load_texture(
                format!("sphere_{}", sphere_key),
                color_img,
                egui::TextureOptions::LINEAR,
            );
            let tid = handle.id();
            self.editor_ctx.asset_thumbnails.insert(sphere_key.clone(), handle);
            Some(tid)
        };

        if let Some(tid) = sphere_id {
            ui.vertical_centered(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(sphere_size, sphere_size),
                    egui::Sense::hover(),
                );
                ui.painter().image(
                    tid,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            });
        }
        ui.add_space(6.0);

        // --- Editable properties ---
        let mut changed = false;
        let path_owned = path.to_path_buf();

        component_section(ui, "mat_props", "M", "Material Properties", Color32::from_rgb(160, 80, 200), false, |ui| {
            property_row(ui, "Name", |ui| {
                ui.label(egui::RichText::new(&mat.name).color(theme::TEXT_PRIMARY));
            });

            property_row(ui, "Albedo", |ui| {
                let mut color = [mat.albedo[0], mat.albedo[1], mat.albedo[2]];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    mat.albedo = color;
                    changed = true;
                }
            });

            property_row(ui, "Roughness", |ui| {
                if ui.add(egui::Slider::new(&mut mat.roughness, 0.0..=1.0)).changed() {
                    changed = true;
                }
            });

            property_row(ui, "Metallic", |ui| {
                if ui.add(egui::Slider::new(&mut mat.metallic, 0.0..=1.0)).changed() {
                    changed = true;
                }
            });

            property_row(ui, "Emission", |ui| {
                let mut color = [
                    mat.emission[0].min(1.0),
                    mat.emission[1].min(1.0),
                    mat.emission[2].min(1.0),
                ];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    mat.emission = color;
                    changed = true;
                }
            });

            if let Some(ref tex) = mat.albedo_texture {
                property_row(ui, "Albedo Tex", |ui| {
                    ui.label(egui::RichText::new(tex).monospace().size(11.0).color(theme::ACCENT));
                });
            }
            if let Some(ref tex) = mat.normal_texture {
                property_row(ui, "Normal Tex", |ui| {
                    ui.label(egui::RichText::new(tex).monospace().size(11.0).color(theme::ACCENT));
                });
            }
        });

        // Auto-save on change
        if changed {
            if let Ok(ron) = ron::ser::to_string_pretty(&mat, ron::ser::PrettyConfig::default()) {
                let _ = std::fs::write(&path_owned, &ron);
                // Invalidate sphere cache so it re-generates with new color
                self.editor_ctx.asset_thumbnails.remove(&sphere_key);
                // Also invalidate the browser thumbnail
                let browser_key = format!("mat_{}", path_owned.to_string_lossy());
                self.editor_ctx.asset_thumbnails.remove(&browser_key);
            }
        }
    }

    // ---- Skeleton (.skel.ron) ----
    fn inspect_skeleton(&self, ui: &mut egui::Ui, path: &Path) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Failed to read: {}", e));
                return;
            }
        };
        let skel: Skeleton = match ron::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Parse error: {}", e));
                return;
            }
        };

        component_section(ui, "skel_props", "S", "Skeleton", Color32::from_rgb(230, 230, 220), false, |ui| {
            property_row(ui, "Name", |ui| {
                ui.label(egui::RichText::new(&skel.name).color(theme::TEXT_PRIMARY));
            });
            property_row(ui, "Bones", |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", skel.bones.len()))
                        .strong()
                        .color(theme::ACCENT),
                );
            });
        });

        // Bone list (collapsible)
        ui.add_space(4.0);
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("Bone Hierarchy ({})", skel.bones.len())).color(theme::TEXT_SECONDARY),
        )
        .default_open(false)
        .show(ui, |ui| {
            for (i, bone) in skel.bones.iter().enumerate() {
                let parent_info = match bone.parent {
                    None => "root".to_string(),
                    Some(p) => format!("parent: {}", p),
                };
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}]", i))
                            .monospace()
                            .size(10.0)
                            .color(theme::TEXT_DISABLED),
                    );
                    ui.label(
                        egui::RichText::new(&bone.name)
                            .size(11.0)
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(format!("({})", parent_info))
                            .size(10.0)
                            .color(theme::TEXT_DISABLED),
                    );
                });
            }
        });
    }

    // ---- Animation Clip (.clip.ron) ----
    fn inspect_clip(&mut self, ui: &mut egui::Ui, path: &Path) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Failed to read: {}", e));
                return;
            }
        };
        let clip: AnimationClip = match ron::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                ui.colored_label(theme::ERROR, format!("Parse error: {}", e));
                return;
            }
        };

        component_section(ui, "clip_props", "A", "Animation Clip", Color32::from_rgb(230, 150, 40), false, |ui| {
            property_row(ui, "Name", |ui| {
                ui.label(egui::RichText::new(&clip.name).color(theme::TEXT_PRIMARY));
            });
            property_row(ui, "Duration", |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.3}s", clip.duration))
                        .strong()
                        .color(theme::ACCENT),
                );
            });
            property_row(ui, "Channels", |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", clip.channels.len()))
                        .color(theme::TEXT_PRIMARY),
                );
            });
        });

        // --- Mini Animation Player ---
        ui.add_space(6.0);
        let duration = clip.duration;
        let accent = Color32::from_rgb(230, 150, 40);

        Frame::NONE
            .fill(theme::BG_MANTLE)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Preview").color(theme::TEXT_SECONDARY).small());

                // Play/Pause + Stop buttons
                ui.horizontal(|ui| {
                    let play_label = if self.editor_ctx.clip_preview_playing { "\u{23F8}" } else { "\u{25B6}" };
                    if ui.button(egui::RichText::new(play_label).size(16.0)).clicked() {
                        self.editor_ctx.clip_preview_playing = !self.editor_ctx.clip_preview_playing;
                    }
                    if ui.button(egui::RichText::new("\u{23F9}").size(16.0)).clicked() {
                        self.editor_ctx.clip_preview_playing = false;
                        self.editor_ctx.clip_preview_time = 0.0;
                    }

                    // Time display
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.2}s / {:.2}s",
                            self.editor_ctx.clip_preview_time, duration
                        ))
                        .monospace()
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY),
                    );
                });

                // Timeline scrub bar
                ui.add_space(2.0);
                let bar_width = ui.available_width();
                let (bar_rect, bar_resp) = ui.allocate_exact_size(
                    egui::vec2(bar_width, 20.0),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter();

                // Background track
                painter.rect_filled(bar_rect, 4.0, theme::BG_SURFACE0);

                // Progress fill
                let progress = if duration > 0.0 {
                    (self.editor_ctx.clip_preview_time / duration).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let fill_rect = egui::Rect::from_min_max(
                    bar_rect.min,
                    egui::pos2(bar_rect.min.x + bar_rect.width() * progress, bar_rect.max.y),
                );
                painter.rect_filled(fill_rect, 4.0, accent);

                // Playhead indicator
                let head_x = bar_rect.min.x + bar_rect.width() * progress;
                let head_center = egui::pos2(head_x, bar_rect.center().y);
                painter.circle_filled(head_center, 5.0, Color32::WHITE);
                painter.circle_stroke(head_center, 5.0, Stroke::new(1.5, accent));

                // Scrub on click/drag
                if bar_resp.clicked() || bar_resp.dragged() {
                    if let Some(pos) = bar_resp.interact_pointer_pos() {
                        let t = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                        self.editor_ctx.clip_preview_time = t * duration;
                    }
                }

                // Advance time if playing
                if self.editor_ctx.clip_preview_playing {
                    let dt = ui.input(|i| i.stable_dt);
                    self.editor_ctx.clip_preview_time += dt;
                    if self.editor_ctx.clip_preview_time >= duration {
                        self.editor_ctx.clip_preview_time = 0.0; // loop
                    }
                    ui.ctx().request_repaint();
                }

                // Keyframe tick marks on the bar
                let unique_times: std::collections::BTreeSet<u32> = clip.channels.iter()
                    .flat_map(|ch| ch.timestamps.iter().map(|t| (*t * 1000.0) as u32))
                    .collect();
                for time_ms in &unique_times {
                    let t = (*time_ms as f32 / 1000.0) / duration;
                    if t >= 0.0 && t <= 1.0 {
                        let tx = bar_rect.min.x + bar_rect.width() * t;
                        painter.line_segment(
                            [egui::pos2(tx, bar_rect.min.y), egui::pos2(tx, bar_rect.min.y + 4.0)],
                            Stroke::new(1.0, Color32::from_white_alpha(100)),
                        );
                    }
                }
            });

        // Channel list (collapsible)
        ui.add_space(4.0);
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("Channels ({})", clip.channels.len())).color(theme::TEXT_SECONDARY),
        )
        .default_open(false)
        .show(ui, |ui| {
            for (i, ch) in clip.channels.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}]", i))
                            .monospace()
                            .size(10.0)
                            .color(theme::TEXT_DISABLED),
                    );
                    ui.label(
                        egui::RichText::new(format!("bone {} {:?}", ch.target_bone, ch.property))
                            .size(11.0)
                            .color(theme::TEXT_PRIMARY),
                    );
                    let keyframes = ch.timestamps.len();
                    ui.label(
                        egui::RichText::new(format!("{} keys", keyframes))
                            .size(10.0)
                            .color(theme::TEXT_DISABLED),
                    );
                });
            }
        });
    }

    // ---- Image (.png, .jpg, etc.) ----
    fn inspect_image(&mut self, ui: &mut egui::Ui, path: &Path) {
        // Show image dimensions + larger preview
        let path_str = path.to_string_lossy().to_string();

        if let Ok(meta) = std::fs::metadata(path) {
            component_section(ui, "img_props", "I", "Image", Color32::from_rgb(60, 60, 60), false, |ui| {
                property_row(ui, "File Size", |ui| {
                    let size = meta.len();
                    let display = if size > 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{} B", size)
                    };
                    ui.label(egui::RichText::new(display).color(theme::TEXT_PRIMARY));
                });
            });
        }

        // Large preview
        ui.add_space(8.0);
        let preview_size = ui.available_width().min(256.0);

        let thumb_id = if let Some(handle) = self.editor_ctx.asset_thumbnails.get(&path_str) {
            Some(handle.id())
        } else {
            if let Ok(dyn_img) = image::open(path) {
                let w = dyn_img.width();
                let h = dyn_img.height();

                // Show dimensions
                ui.label(
                    egui::RichText::new(format!("{}x{}", w, h))
                        .monospace()
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY),
                );

                let thumb = dyn_img
                    .thumbnail(preview_size as u32, preview_size as u32)
                    .into_rgba8();
                let size = [thumb.width() as usize, thumb.height() as usize];
                let pixels: Vec<egui::Color32> = thumb
                    .pixels()
                    .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                let color_img = egui::ColorImage {
                    size,
                    source_size: egui::Vec2::new(size[0] as f32, size[1] as f32),
                    pixels,
                };
                let handle = ui.ctx().load_texture(
                    format!("preview_{}", path_str),
                    color_img,
                    egui::TextureOptions::LINEAR,
                );
                let tid = handle.id();
                self.editor_ctx
                    .asset_thumbnails
                    .insert(path_str.clone(), handle);
                Some(tid)
            } else {
                None
            }
        };

        if let Some(tid) = thumb_id {
            // Centered preview with border
            ui.vertical_centered(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(preview_size, preview_size), egui::Sense::hover());
                // Checkerboard background for transparency
                let painter = ui.painter();
                painter.rect_filled(rect, 4.0, Color32::from_gray(40));
                painter.image(
                    tid,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                painter.rect_stroke(rect, 4.0, Stroke::new(1.0, theme::BG_SURFACE1), egui::StrokeKind::Outside);
            });
        }
    }

    // ---- Mesh file (.fbx, .glb, .gltf, .obj) ----
    fn inspect_mesh_file(&self, ui: &mut egui::Ui, path: &Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        component_section(ui, "mesh_file", "3D", "3D Model", Color32::from_rgb(60, 160, 70), false, |ui| {
            property_row(ui, "Format", |ui| {
                ui.label(
                    egui::RichText::new(ext.to_uppercase())
                        .strong()
                        .color(theme::ACCENT),
                );
            });
            property_row(ui, "Action", |ui| {
                ui.label(
                    egui::RichText::new("Double-click to import")
                        .size(11.0)
                        .color(theme::TEXT_DISABLED),
                );
            });
        });

        // Check for extracted sub-assets
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent_dir = path.parent().unwrap_or(Path::new("."));
        let marker = parent_dir.join(format!("{}_.extracted", stem));
        if marker.exists() {
            ui.add_space(4.0);
            component_section(ui, "extracted", "E", "Extracted Sub-Assets", theme::SUCCESS, false, |ui| {
                // List extracted files
                if let Ok(entries) = std::fs::read_dir(parent_dir) {
                    let prefix = format!("{}_", stem);
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with(&prefix) && !name.ends_with(".extracted") {
                            let icon = if name.ends_with(".skel.ron") {
                                "\u{1F9B4}" // bone
                            } else if name.ends_with(".clip.ron") {
                                "\u{1F3AC}" // clapper
                            } else if name.ends_with(".mat.ron") {
                                "\u{26AA}" // circle
                            } else if name.ends_with(".png") {
                                "\u{1F5BC}" // framed picture
                            } else {
                                "\u{1F4C4}" // page
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(icon).size(12.0));
                                ui.label(
                                    egui::RichText::new(&name)
                                        .size(11.0)
                                        .color(theme::TEXT_SECONDARY),
                                );
                            });
                        }
                    }
                }
            });
        }
    }

    // ---- Scene (.scene.ron) ----
    fn inspect_scene_file(&self, ui: &mut egui::Ui, _path: &Path) {
        component_section(ui, "scene_file", "Sc", "Scene", Color32::from_rgb(55, 55, 65), false, |ui| {
            property_row(ui, "Type", |ui| {
                ui.label(egui::RichText::new("Scene File").color(theme::TEXT_PRIMARY));
            });
            property_row(ui, "Action", |ui| {
                ui.label(
                    egui::RichText::new("Double-click to load")
                        .size(11.0)
                        .color(theme::TEXT_DISABLED),
                );
            });
        });
    }

    // ---- Script (.rs) ----
    fn inspect_script_file(&self, ui: &mut egui::Ui, path: &Path) {
        component_section(ui, "script_file", "Rs", "Script", Color32::from_rgb(50, 120, 210), false, |ui| {
            property_row(ui, "Language", |ui| {
                ui.label(egui::RichText::new("Rust").strong().color(theme::ACCENT));
            });
        });

        // Show first ~20 lines as preview
        if let Ok(content) = std::fs::read_to_string(path) {
            ui.add_space(4.0);
            egui::CollapsingHeader::new(
                egui::RichText::new("Preview").color(theme::TEXT_SECONDARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                let preview: String = content.lines().take(20).collect::<Vec<_>>().join("\n");
                Frame::NONE
                    .fill(theme::BG_CRUST)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&preview)
                                .monospace()
                                .size(10.0)
                                .color(theme::TEXT_SECONDARY),
                        );
                    });
                if content.lines().count() > 20 {
                    ui.label(
                        egui::RichText::new("...")
                            .color(theme::TEXT_DISABLED)
                            .size(10.0),
                    );
                }
            });
        }
    }

    // ---- Audio (.wav, .mp3, .ogg) ----
    fn inspect_audio_file(&self, ui: &mut egui::Ui, path: &Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        component_section(ui, "audio_file", "Au", "Audio", Color32::from_rgb(60, 190, 190), false, |ui| {
            property_row(ui, "Format", |ui| {
                ui.label(
                    egui::RichText::new(ext.to_uppercase())
                        .strong()
                        .color(theme::ACCENT),
                );
            });
        });
    }

    // ---- Generic file ----
    fn inspect_generic_file(&self, ui: &mut egui::Ui, path: &Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or("unknown".to_string());

        component_section(ui, "generic_file", "F", "File", Color32::from_rgb(140, 140, 140), false, |ui| {
            property_row(ui, "Extension", |ui| {
                ui.label(egui::RichText::new(&ext).color(theme::TEXT_PRIMARY));
            });
        });
    }

    // ---- Multi-asset selection summary ----
    fn show_multi_asset_inspector(&self, ui: &mut egui::Ui) {
        let count = self.editor_ctx.selected_assets.len();

        ui.horizontal(|ui| {
            ui.strong("Inspector");
            ui.label(
                egui::RichText::new(format!(" {} Assets", count))
                    .small()
                    .strong()
                    .color(theme::ACCENT)
                    .background_color(theme::ACCENT.gamma_multiply(0.15)),
            );
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Summary card
            Frame::NONE
                .fill(theme::BG_MANTLE)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 6))
                .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} assets selected", count))
                            .strong()
                            .color(theme::ACCENT)
                            .size(14.0),
                    );

                    // Count by type
                    let mut mats = 0;
                    let mut clips = 0;
                    let mut skels = 0;
                    let mut images = 0;
                    let mut meshes = 0;
                    let mut other = 0;
                    let mut total_size: u64 = 0;

                    for p in &self.editor_ctx.selected_assets {
                        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        if name.ends_with(".mat.ron") { mats += 1; }
                        else if name.ends_with(".clip.ron") { clips += 1; }
                        else if name.ends_with(".skel.ron") { skels += 1; }
                        else if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") { images += 1; }
                        else if name.ends_with(".fbx") || name.ends_with(".glb") || name.ends_with(".gltf") || name.ends_with(".obj") { meshes += 1; }
                        else { other += 1; }
                        if let Ok(meta) = std::fs::metadata(p) {
                            total_size += meta.len();
                        }
                    }

                    ui.add_space(4.0);

                    // Type breakdown
                    let types: Vec<(&str, usize, Color32)> = vec![
                        ("Materials", mats, Color32::from_rgb(160, 80, 200)),
                        ("Clips", clips, Color32::from_rgb(230, 150, 40)),
                        ("Skeletons", skels, Color32::from_rgb(230, 230, 220)),
                        ("Images", images, Color32::from_rgb(100, 100, 100)),
                        ("3D Models", meshes, Color32::from_rgb(60, 160, 70)),
                        ("Other", other, Color32::from_rgb(140, 140, 140)),
                    ];

                    for (label, n, color) in &types {
                        if *n > 0 {
                            ui.horizontal(|ui| {
                                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 4.0, *color);
                                ui.label(
                                    egui::RichText::new(format!("{} {}", n, label))
                                        .size(12.0)
                                        .color(theme::TEXT_PRIMARY),
                                );
                            });
                        }
                    }

                    // Total size
                    ui.add_space(4.0);
                    let size_display = if total_size > 1_048_576 {
                        format!("{:.1} MB total", total_size as f64 / 1_048_576.0)
                    } else if total_size > 1024 {
                        format!("{:.1} KB total", total_size as f64 / 1024.0)
                    } else {
                        format!("{} B total", total_size)
                    };
                    ui.label(
                        egui::RichText::new(size_display)
                            .color(theme::TEXT_DISABLED)
                            .size(11.0),
                    );
                });

            // File list
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                egui::RichText::new("Selected Files").color(theme::TEXT_SECONDARY),
            )
            .default_open(true)
            .show(ui, |ui| {
                for p in &self.editor_ctx.selected_assets.clone() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let (icon, color) = asset_type_icon(&name);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).color(color).size(12.0));
                        ui.label(
                            egui::RichText::new(&name)
                                .size(11.0)
                                .color(theme::TEXT_PRIMARY),
                        );
                    });
                }
            });
        });
    }
}

/// Returns (icon_char, color) for a given file name.
fn asset_type_icon(name: &str) -> (&'static str, Color32) {
    if name.ends_with(".mat.ron") {
        ("\u{26AA}", Color32::from_rgb(160, 80, 200))      // material = purple
    } else if name.ends_with(".skel.ron") {
        ("\u{1F9B4}", Color32::from_rgb(230, 230, 220))    // skeleton = bone white
    } else if name.ends_with(".clip.ron") {
        ("\u{1F3AC}", Color32::from_rgb(230, 150, 40))     // clip = orange
    } else if name.ends_with(".scene.ron") {
        ("\u{1F3AC}", Color32::from_rgb(55, 55, 65))       // scene = dark
    } else if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
        ("\u{1F5BC}", Color32::from_rgb(100, 100, 100))    // image = gray
    } else if name.ends_with(".fbx") || name.ends_with(".glb") || name.ends_with(".gltf") || name.ends_with(".obj") {
        ("\u{1F4E6}", Color32::from_rgb(60, 160, 70))      // mesh = green
    } else if name.ends_with(".rs") {
        ("\u{1F4DC}", Color32::from_rgb(50, 120, 210))     // script = blue
    } else if name.ends_with(".wav") || name.ends_with(".mp3") || name.ends_with(".ogg") {
        ("\u{1F50A}", Color32::from_rgb(60, 190, 190))     // audio = teal
    } else {
        ("\u{1F4C4}", Color32::from_rgb(140, 140, 140))    // generic = gray
    }
}
