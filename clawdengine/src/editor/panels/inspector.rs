use egui::{ComboBox, CornerRadius, DragValue, Frame, Margin, Slider, Stroke};
use glam::EulerRot;

use crate::core::LightKind;
use crate::editor::context::ComponentKind;
use crate::editor::layout::{
    EditorTabViewer, TextureSlotAction, component_section, property_row,
    sanitize_vec3, texture_slot, vec3_drag, vec3_drag_array,
};
use crate::editor::theme;

// Component accent colors (sourced from theme module — single source of truth).
const MAT_ACCENT: egui::Color32 = theme::COMPONENT_MATERIAL;
const MESH_ACCENT: egui::Color32 = theme::COMPONENT_MESH;
const RB_ACCENT: egui::Color32 = theme::COMPONENT_RIGIDBODY;

impl<'a> EditorTabViewer<'a> {
    pub(crate) fn show_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Inspector");
            if self.editor_ctx.play_mode {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(" Play Mode ")
                        .small()
                        .strong()
                        .color(theme::SUCCESS)
                        .background_color(theme::SUCCESS.gamma_multiply(0.15)),
                );
            }
        });
        ui.separator();

        if self.editor_ctx.selected_entities.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("\u{1F50D}")
                        .size(28.0)
                        .color(theme::TEXT_DISABLED),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Select an entity to inspect")
                        .color(theme::TEXT_DISABLED)
                        .size(13.0),
                );
            });
            return;
        }

        if self.editor_ctx.selected_entities.len() > 1 {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(format!(
                        "\u{2610} {} entities selected",
                        self.editor_ctx.selected_entities.len()
                    ))
                    .strong()
                    .color(theme::ACCENT),
                );
            });
            return;
        }

        let eid = self.editor_ctx.selected_entities[0];

        // Push undo snapshot when user starts interacting with inspector widgets
        if !self.editor_ctx.play_mode && !self.editor_ctx.inspector_editing {
            let pointer_pressed = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
            if pointer_pressed && ui.rect_contains_pointer(ui.max_rect()) {
                self.editor_ctx.undo_stack.push(
                    self.world.snapshot(),
                    self.editor_ctx.selected_entities.clone(),
                );
                self.editor_ctx.inspector_editing = true;
            }
        }

        let play_mode = self.editor_ctx.play_mode;
        egui::ScrollArea::vertical().show(ui, |ui| {
        // In play mode the World is driven by scripts/physics; any inspector
        // edit would be silently discarded on stop. Disable the whole body so
        // the user can still read values but not lose work editing them.
        if play_mode {
            ui.disable();
        }
        // ---- Entity Header ----
        let (icon, icon_color) = crate::editor::layout::entity_icon(self.world, eid);
        Frame::NONE
            .fill(theme::BG_MANTLE)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .stroke(Stroke::new(1.0, theme::BG_SURFACE0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).color(icon_color).size(18.0));
                    if let Some(name) = self.world.get_name_mut(eid) {
                        ui.add(
                            egui::TextEdit::singleline(name)
                                .font(egui::TextStyle::Body)
                                .desired_width(ui.available_width() - 50.0),
                        );
                    }
                });
                ui.label(
                    egui::RichText::new(format!("ID: {}", eid.index))
                        .color(theme::TEXT_DISABLED)
                        .size(10.0),
                );
            });
        ui.add_space(6.0);

        // ---- Transform ----
        if self.world.get_transform(eid).is_some() {
            component_section(ui, "transform", "T", "Transform", theme::ACCENT, false, |ui| {
                let Some(t) = self.world.get_transform_mut(eid) else { return; };

                ui.label(egui::RichText::new("Position").color(theme::TEXT_DISABLED).small());
                vec3_drag(ui, &mut t.position, 0.05);

                let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
                let mut deg = [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()];
                ui.label(egui::RichText::new("Rotation").color(theme::TEXT_DISABLED).small());
                if vec3_drag_array(ui, &mut deg, 0.5) {
                    t.rotation = glam::Quat::from_euler(
                        EulerRot::XYZ,
                        deg[0].to_radians(),
                        deg[1].to_radians(),
                        deg[2].to_radians(),
                    );
                }

                ui.label(egui::RichText::new("Scale").color(theme::TEXT_DISABLED).small());
                vec3_drag(ui, &mut t.scale, 0.05);

                // Guard against NaN/Inf (paste, runaway drag) and a zero scale
                // which would collapse the model matrix and break picking.
                sanitize_vec3(&mut t.position, 0.0);
                sanitize_vec3(&mut t.scale, 1.0);
                const MIN_SCALE: f32 = 1.0e-3;
                if t.scale.x.abs() < MIN_SCALE { t.scale.x = MIN_SCALE; }
                if t.scale.y.abs() < MIN_SCALE { t.scale.y = MIN_SCALE; }
                if t.scale.z.abs() < MIN_SCALE { t.scale.z = MIN_SCALE; }
            });
        }

        // ---- Material ----
        if self.world.get_material(eid).is_some() {
            let remove_mat = component_section(ui, "material", "M", "Material", MAT_ACCENT, true, |ui| {
                let Some(m) = self.world.get_material_mut(eid) else { return; };

                // Surface properties
                ui.label(egui::RichText::new("Surface").color(theme::TEXT_DISABLED).small());
                property_row(ui, "Color", |ui| {
                    let mut color = [m.albedo.x, m.albedo.y, m.albedo.z];
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        m.albedo = glam::Vec3::new(color[0], color[1], color[2]);
                    }
                });
                property_row(ui, "Roughness", |ui| {
                    ui.add(Slider::new(&mut m.roughness, 0.0..=1.0).show_value(true));
                });
                property_row(ui, "Metallic", |ui| {
                    ui.add(Slider::new(&mut m.metallic, 0.0..=1.0).show_value(true));
                });

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Emission").color(theme::TEXT_DISABLED).small());
                property_row(ui, "Color", |ui| {
                    let mut em = [m.emission.x, m.emission.y, m.emission.z];
                    if ui.color_edit_button_rgb(&mut em).changed() {
                        m.emission = glam::Vec3::new(em[0], em[1], em[2]);
                    }
                });

                // Textures
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Textures").color(theme::TEXT_DISABLED).small());

                // Albedo texture slot
                let albedo_action = texture_slot(
                    ui,
                    "Albedo",
                    &m.texture_path,
                    theme::ACCENT,
                );
                if albedo_action == TextureSlotAction::Remove {
                    m.texture_path = None;
                    m.texture_id = None;
                }

                // Albedo browse button + popup
                let albedo_btn = ui.small_button("Browse Texture...");
                egui::Popup::from_toggle_button_response(&albedo_btn)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui: &mut egui::Ui| {
                        ui.set_min_width(200.0);
                        Self::texture_browser(ui, eid, false, &mut self.editor_ctx.pending_texture_assign);
                    });

                ui.add_space(2.0);

                // Normal map slot
                let normal_action = texture_slot(
                    ui,
                    "Normal Map",
                    &m.normal_map_path,
                    theme::TEAL,
                );
                if normal_action == TextureSlotAction::Remove {
                    m.normal_map_path = None;
                    m.normal_map_id = None;
                }

                // Normal browse button + popup
                let nmap_btn = ui.small_button("Browse Normal Map...");
                egui::Popup::from_toggle_button_response(&nmap_btn)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui: &mut egui::Ui| {
                        ui.set_min_width(200.0);
                        Self::texture_browser(ui, eid, true, &mut self.editor_ctx.pending_normal_map_assign);
                    });
            });
            if remove_mat {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_material(eid);
            }
        }

        // ---- Light ----
        if self.world.get_light(eid).is_some() {
            let remove_light = component_section(ui, "light", "L", "Light", theme::WARNING, true, |ui| {
                let Some(l) = self.world.get_light_mut(eid) else { return; };

                property_row(ui, "Kind", |ui| {
                    ComboBox::from_id_salt("light_kind")
                        .selected_text(format!("{:?}", l.kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut l.kind, LightKind::Directional, "Directional");
                            ui.selectable_value(&mut l.kind, LightKind::Point, "Point");
                            ui.selectable_value(&mut l.kind, LightKind::Spot, "Spot");
                        });
                });
                property_row(ui, "Color", |ui| {
                    let mut color = [l.color.x, l.color.y, l.color.z];
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        l.color = glam::Vec3::new(color[0], color[1], color[2]);
                    }
                });
                property_row(ui, "Intensity", |ui| {
                    ui.add(DragValue::new(&mut l.intensity).speed(0.05));
                });
                property_row(ui, "Range", |ui| {
                    ui.add(DragValue::new(&mut l.range).speed(0.1).range(0.1..=100.0));
                });

                if l.kind == LightKind::Spot {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Spot Cone").color(theme::TEXT_DISABLED).small());
                    let mut inner_deg = l.inner_angle.to_degrees();
                    let mut outer_deg = l.outer_angle.to_degrees();
                    property_row(ui, "Inner", |ui| {
                        if ui.add(Slider::new(&mut inner_deg, 1.0..=89.0)).changed() {
                            l.inner_angle = inner_deg.to_radians();
                            if l.inner_angle > l.outer_angle {
                                l.outer_angle = l.inner_angle;
                            }
                        }
                    });
                    property_row(ui, "Outer", |ui| {
                        if ui.add(Slider::new(&mut outer_deg, 1.0..=89.0)).changed() {
                            l.outer_angle = outer_deg.to_radians();
                            if l.outer_angle < l.inner_angle {
                                l.inner_angle = l.outer_angle;
                            }
                        }
                    });
                }
            });
            if remove_light {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_light(eid);
            }
        }

        // ---- MeshRenderer ----
        if self.world.get_mesh_renderer(eid).is_some() {
            let remove_mr = component_section(ui, "mesh_renderer", "R", "MeshRenderer", MESH_ACCENT, true, |ui| {
                let Some(mr) = self.world.get_mesh_renderer_mut(eid) else { return; };
                property_row(ui, "Visible", |ui| {
                    ui.checkbox(&mut mr.visible, "");
                });
                property_row(ui, "Mesh", |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:?}", mr.mesh_id))
                            .color(theme::TEXT_SECONDARY)
                            .monospace()
                            .size(11.0),
                    );
                });
            });
            if remove_mr {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_mesh_renderer(eid);
            }
        }

        // ---- RigidBody ----
        if self.world.get_rigid_body(eid).is_some() {
            let remove_rb = component_section(ui, "rigid_body", "P", "RigidBody", RB_ACCENT, true, |ui| {
                let Some(rb) = self.world.get_rigid_body_mut(eid) else { return; };
                property_row(ui, "Mass", |ui| {
                    ui.add(DragValue::new(&mut rb.mass).speed(0.1).range(0.01..=f32::MAX));
                });
                property_row(ui, "Gravity", |ui| {
                    ui.checkbox(&mut rb.gravity_enabled, "");
                });

                ui.add_space(2.0);
                ui.label(egui::RichText::new("Velocity").color(theme::TEXT_DISABLED).small());
                vec3_drag(ui, &mut rb.velocity, 0.1);
                ui.label(egui::RichText::new("Angular Vel.").color(theme::TEXT_DISABLED).small());
                vec3_drag(ui, &mut rb.angular_velocity, 0.1);
            });
            if remove_rb {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_rigid_body(eid);
            }
        }

        // ---- Collider ----
        if self.world.get_collider(eid).is_some() {
            let remove_col = component_section(ui, "collider", "C", "Collider", theme::ACCENT, true, |ui| {
                let Some(col) = self.world.get_collider_mut(eid) else { return; };
                property_row(ui, "Shape", |ui| {
                    egui::ComboBox::from_id_salt("collider_shape")
                        .selected_text(match col.shape {
                            crate::core::ColliderShape::Box => "Box",
                            crate::core::ColliderShape::Sphere => "Sphere",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut col.shape, crate::core::ColliderShape::Box, "Box");
                            ui.selectable_value(&mut col.shape, crate::core::ColliderShape::Sphere, "Sphere");
                        });
                });
                property_row(ui, "Center", |ui| {
                    vec3_drag(ui, &mut col.center, 0.01);
                });
                match col.shape {
                    crate::core::ColliderShape::Box => {
                        property_row(ui, "Half Extents", |ui| {
                            vec3_drag(ui, &mut col.half_extents, 0.01);
                        });
                    }
                    crate::core::ColliderShape::Sphere => {
                        property_row(ui, "Radius", |ui| {
                            ui.add(DragValue::new(&mut col.radius).speed(0.01).range(0.01..=100.0));
                        });
                    }
                }
                property_row(ui, "Restitution", |ui| {
                    ui.add(DragValue::new(&mut col.restitution).speed(0.01).range(0.0..=1.0));
                });
                property_row(ui, "Friction", |ui| {
                    ui.add(DragValue::new(&mut col.friction).speed(0.01).range(0.0..=1.0));
                });
                property_row(ui, "Trigger", |ui| {
                    ui.checkbox(&mut col.is_trigger, "");
                });
            });
            if remove_col {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_collider(eid);
            }
        }

        // ---- Camera ----
        if self.world.get_camera(eid).is_some() {
            let remove_cam = component_section(ui, "camera", "Cam", "Camera", theme::SKY, true, |ui| {
                let Some(cam) = self.world.get_camera_mut(eid) else { return; };
                property_row(ui, "FOV (deg)", |ui| {
                    let mut fov_deg = cam.fov_y.to_degrees();
                    if ui.add(DragValue::new(&mut fov_deg).speed(0.5).range(10.0..=160.0)).changed() {
                        cam.fov_y = fov_deg.to_radians();
                    }
                });
                property_row(ui, "Near", |ui| {
                    ui.add(DragValue::new(&mut cam.near).speed(0.01).range(0.001..=100.0));
                });
                property_row(ui, "Far", |ui| {
                    ui.add(DragValue::new(&mut cam.far).speed(1.0).range(1.0..=10000.0));
                });
                property_row(ui, "Main Camera", |ui| {
                    ui.checkbox(&mut cam.is_main, "");
                });
            });
            if remove_cam {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_camera(eid);
            }
        }

        // ---- AudioListener ----
        if self.world.get_audio_listener(eid).is_some() {
            let remove_al = component_section(ui, "audio_listener", "AL", "AudioListener", theme::MAUVE, true, |ui| {
                let Some(al) = self.world.get_audio_listener_mut(eid) else { return; };
                property_row(ui, "Active", |ui| {
                    ui.checkbox(&mut al.active, "");
                });
                property_row(ui, "Volume", |ui| {
                    ui.add(Slider::new(&mut al.volume, 0.0..=1.0));
                });
            });
            if remove_al {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_audio_listener(eid);
            }
        }

        // ---- AudioSource ----
        if self.world.get_audio_source(eid).is_some() {
            let remove_audio = component_section(ui, "audio", "A", "AudioSource", theme::WARNING, true, |ui| {
                let Some(audio) = self.world.get_audio_source_mut(eid) else { return; };

                property_row(ui, "File", |ui| {
                    let mut remove = false;
                    if let Some(ref path) = audio.audio_path {
                        let display = path.rsplit('/').next().unwrap_or(path);
                        ui.label(egui::RichText::new(display).color(theme::TEXT_PRIMARY).size(11.0));
                        if ui.small_button("x").on_hover_text("Remove").clicked() {
                            remove = true;
                        }
                    } else {
                        ui.label(egui::RichText::new("None").color(theme::TEXT_DISABLED).size(11.0));
                    }
                    if remove { audio.audio_path = None; }
                });

                let browse = ui.small_button("Browse Audio...");
                egui::Popup::from_toggle_button_response(&browse)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui: &mut egui::Ui| {
                        ui.set_min_width(200.0);
                        fn scan(dir: &std::path::Path, files: &mut Vec<String>) {
                            if let Ok(entries) = std::fs::read_dir(dir) {
                                for entry in entries.flatten() {
                                    let p = entry.path();
                                    if p.is_dir() { scan(&p, files); }
                                    else if let Some(ext) = p.extension() {
                                        let ext = ext.to_string_lossy().to_lowercase();
                                        if ext == "wav" || ext == "ogg" {
                                            files.push(p.to_string_lossy().into_owned());
                                        }
                                    }
                                }
                            }
                        }
                        let mut files = Vec::new();
                        scan(std::path::Path::new("assets"), &mut files);
                        files.sort();
                        if files.is_empty() {
                            ui.label(egui::RichText::new("No .wav/.ogg files").color(theme::TEXT_DISABLED));
                        }
                        for path in &files {
                            let display = path.rsplit('/').next().unwrap_or(path);
                            if ui.button(display).clicked() {
                                audio.audio_path = Some(path.clone());
                            }
                        }
                    });

                property_row(ui, "Volume", |ui| {
                    ui.add(Slider::new(&mut audio.volume, 0.0..=1.0));
                });
                property_row(ui, "Pitch", |ui| {
                    ui.add(DragValue::new(&mut audio.pitch).speed(0.01).range(0.5..=2.0));
                });
                property_row(ui, "Loop", |ui| {
                    ui.checkbox(&mut audio.loop_audio, "");
                });
                property_row(ui, "Play on Start", |ui| {
                    ui.checkbox(&mut audio.play_on_start, "");
                });
                property_row(ui, "Spatial", |ui| {
                    ui.checkbox(&mut audio.spatial, "");
                });
                if audio.spatial {
                    property_row(ui, "Max Distance", |ui| {
                        ui.add(DragValue::new(&mut audio.max_distance).speed(0.5).range(0.1..=100.0));
                    });
                }
            });
            if remove_audio {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_audio_source(eid);
            }
        }

        // ---- Canvas ----
        if self.world.get_canvas(eid).is_some() {
            let remove_cv = component_section(ui, "canvas", "Cv", "Canvas", theme::MAUVE, true, |ui| {
                let Some(cv) = self.world.get_canvas_mut(eid) else { return; };
                property_row(ui, "Width", |ui| {
                    ui.add(DragValue::new(&mut cv.width).speed(1.0).range(100.0..=3840.0));
                });
                property_row(ui, "Height", |ui| {
                    ui.add(DragValue::new(&mut cv.height).speed(1.0).range(100.0..=2160.0));
                });
                property_row(ui, "Visible", |ui| {
                    ui.checkbox(&mut cv.visible, "");
                });
            });
            if remove_cv {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_canvas(eid);
            }
        }

        // ---- UiElement ----
        if self.world.get_ui_element(eid).is_some() {
            let remove_ui = component_section(ui, "ui_element", "U", "UiElement", theme::MAUVE, true, |ui| {
                let Some(el) = self.world.get_ui_element_mut(eid) else { return; };

                property_row(ui, "Kind", |ui| {
                    ComboBox::from_id_salt("ui_kind")
                        .selected_text(format!("{:?}", el.kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut el.kind, crate::core::UiElementKind::Text, "Text");
                            ui.selectable_value(&mut el.kind, crate::core::UiElementKind::Panel, "Panel");
                        });
                });

                if el.kind == crate::core::UiElementKind::Text {
                    property_row(ui, "Text", |ui| {
                        ui.add(egui::TextEdit::singleline(&mut el.text).desired_width(120.0));
                    });
                    property_row(ui, "Font Size", |ui| {
                        ui.add(DragValue::new(&mut el.font_size).speed(0.5).range(6.0..=72.0));
                    });
                }

                property_row(ui, "Color", |ui| {
                    let mut color = [el.color.x, el.color.y, el.color.z];
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        el.color = glam::Vec3::new(color[0], color[1], color[2]);
                    }
                });
                property_row(ui, "Alpha", |ui| {
                    ui.add(Slider::new(&mut el.alpha, 0.0..=1.0));
                });
                property_row(ui, "Anchor", |ui| {
                    ComboBox::from_id_salt("ui_anchor")
                        .selected_text(format!("{:?}", el.anchor))
                        .show_ui(ui, |ui| {
                            use crate::core::UiAnchor::*;
                            for a in [TopLeft, TopCenter, TopRight, CenterLeft, Center, CenterRight, BottomLeft, BottomCenter, BottomRight] {
                                ui.selectable_value(&mut el.anchor, a, format!("{:?}", a));
                            }
                        });
                });
                property_row(ui, "Offset", |ui| {
                    ui.add(DragValue::new(&mut el.offset[0]).speed(1.0).prefix("X: "));
                    ui.add(DragValue::new(&mut el.offset[1]).speed(1.0).prefix("Y: "));
                });

                if el.kind == crate::core::UiElementKind::Panel {
                    property_row(ui, "Size", |ui| {
                        ui.add(DragValue::new(&mut el.size[0]).speed(1.0).prefix("W: ").range(1.0..=2000.0));
                        ui.add(DragValue::new(&mut el.size[1]).speed(1.0).prefix("H: ").range(1.0..=2000.0));
                    });
                }

                property_row(ui, "Visible", |ui| {
                    ui.checkbox(&mut el.visible, "");
                });
            });
            if remove_ui {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_ui_element(eid);
            }
        }

        // ---- Scripts ----
        let has_scripts = self.scripts.iter().any(|(id, _)| *id == eid);
        if has_scripts {
            component_section(ui, "scripts", "S", "Scripts", theme::SUCCESS, false, |ui| {
                for (sid, script) in self.scripts.iter_mut() {
                    if *sid == eid {
                        let sname = script.name().to_string();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(">")
                                    .color(theme::SUCCESS)
                                    .monospace()
                                    .size(10.0),
                            );
                            ui.strong(&sname);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("x").size(10.0).color(theme::TEXT_DISABLED),
                                    )
                                    .frame(false),
                                );
                                if btn.on_hover_text("Remove script").clicked() {
                                    self.editor_ctx.pending_remove_scripts.push((*sid, sname.clone()));
                                }
                            });
                        });
                        ui.indent(&sname, |ui| {
                            script.inspector_ui(ui);
                        });
                    }
                }
            });
        }

        // ---- Add Script dropdown ----
        if !self.editor_ctx.script_registry.is_empty() {
            ui.add_space(4.0);
            let registry_names: Vec<&str> = self
                .editor_ctx
                .script_registry
                .iter()
                .map(|e| e.name.as_str())
                .collect();
            ComboBox::from_label("Add Script")
                .selected_text("Select...")
                .show_ui(ui, |ui| {
                    for (i, name) in registry_names.iter().enumerate() {
                        if ui.selectable_label(false, *name).clicked() {
                            self.editor_ctx.pending_add_script = Some((eid, i));
                        }
                    }
                });
        }

        // ---- Add Component button ----
        ui.add_space(8.0);
        let has_mat = self.world.get_material(eid).is_some();
        let has_mr = self.world.get_mesh_renderer(eid).is_some();
        let has_rb = self.world.get_rigid_body(eid).is_some();
        let has_col = self.world.get_collider(eid).is_some();
        let has_cam = self.world.get_camera(eid).is_some();
        let has_audio = self.world.get_audio_source(eid).is_some();
        let has_al = self.world.get_audio_listener(eid).is_some();
        let has_ui = self.world.get_ui_element(eid).is_some();
        let has_cv = self.world.get_canvas(eid).is_some();
        let all_present = has_mat && has_mr && has_rb && has_col && has_cam && has_audio && has_al && has_ui && has_cv;

        let btn = egui::Button::new(
            egui::RichText::new("+ Add Component")
                .color(theme::TEXT_PRIMARY)
                .size(13.0),
        )
        .fill(theme::ACCENT.gamma_multiply(0.15))
        .stroke(Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.3)))
        .corner_radius(CornerRadius::same(6))
        .min_size(egui::vec2(ui.available_width(), 28.0));

        let btn_resp = ui.add(btn);

        egui::Popup::from_toggle_button_response(&btn_resp)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui: &mut egui::Ui| {
                ui.set_min_width(180.0);
                if all_present {
                    ui.label(egui::RichText::new("All components added").color(theme::TEXT_DISABLED));
                } else {
                    if !has_mat && ui.button("Material").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::Material));
                    }
                    if !has_mr && ui.button("MeshRenderer").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::MeshRenderer));
                    }
                    if !has_rb && ui.button("RigidBody").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::RigidBody));
                    }
                    if !has_col && ui.button("Collider").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::Collider));
                    }
                    let has_cam = self.world.get_camera(eid).is_some();
                    if !has_cam && ui.button("Camera").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::CameraComponent));
                    }
                    let has_audio = self.world.get_audio_source(eid).is_some();
                    if !has_audio && ui.button("AudioSource").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::AudioSource));
                    }
                    let has_al = self.world.get_audio_listener(eid).is_some();
                    if !has_al && ui.button("AudioListener").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::AudioListener));
                    }
                    let has_ui = self.world.get_ui_element(eid).is_some();
                    if !has_ui && ui.button("UiElement").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::UiElement));
                    }
                    let has_cv = self.world.get_canvas(eid).is_some();
                    if !has_cv && ui.button("Canvas").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::Canvas));
                    }
                }
            });

        }); // ScrollArea

        // Reset editing flag when pointer is released
        if self.editor_ctx.inspector_editing {
            let pointer_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
            if !pointer_down {
                self.editor_ctx.inspector_editing = false;
            }
        }
    }

    /// Shared texture browser popup content
    fn texture_browser(
        ui: &mut egui::Ui,
        eid: crate::core::EntityId,
        _is_normal: bool,
        target: &mut Option<(crate::core::EntityId, String)>,
    ) {
        let mut found_any = false;
        if let Ok(entries) = std::fs::read_dir("assets/textures") {
            let mut files: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let ext = p.extension()?.to_string_lossy().to_lowercase();
                    if ext == "png" || ext == "jpg" || ext == "jpeg" {
                        Some(p.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect();
            files.sort();
            for file in &files {
                let display = file.rsplit('/').next().unwrap_or(file);
                if ui.button(display).clicked() {
                    *target = Some((eid, file.clone()));
                }
                found_any = true;
            }
        }
        if !found_any {
            ui.label(
                egui::RichText::new("No textures in assets/textures/")
                    .color(theme::TEXT_DISABLED),
            );
        }
    }
}
