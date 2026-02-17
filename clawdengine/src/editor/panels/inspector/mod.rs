mod animation_sections;
mod asset_inspector;
mod audio_section;
mod material_section;
mod physics_sections;
mod script_section;
mod ui_section;

use egui::{Color32, CornerRadius, DragValue, Frame, Margin, Stroke};
use glam::EulerRot;

use crate::core::LightKind;
use crate::editor::layout::{
    EditorTabViewer, axis_drag, component_section, property_row,
};
use crate::editor::theme;

const MESH_ACCENT: Color32 = Color32::from_rgb(0x94, 0xE2, 0xD5); // teal

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
            // Show bone inspector if a bone is selected
            if self.editor_ctx.selected_bone.is_some() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.inspector_bone(ui);
                });
                return;
            }
            // Show asset inspector if asset file(s) selected in the browser
            if !self.editor_ctx.selected_assets.is_empty() {
                self.show_asset_inspector(ui);
                return;
            }
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

        egui::ScrollArea::vertical().show(ui, |ui| {
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
            let col_r = theme::AXIS_X;
            let col_g = theme::AXIS_Y;
            let col_b = theme::AXIS_Z;
            component_section(ui, "transform", "T", "Transform", theme::ACCENT, false, |ui| {
                let Some(t) = self.world.get_transform_mut(eid) else { return; };
                ui.label(egui::RichText::new("Position").color(theme::TEXT_DISABLED).small());
                ui.horizontal(|ui| {
                    axis_drag(ui, "X", col_r, &mut t.position.x, 0.05);
                    axis_drag(ui, "Y", col_g, &mut t.position.y, 0.05);
                    axis_drag(ui, "Z", col_b, &mut t.position.z, 0.05);
                });
                let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
                let mut deg = [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()];
                ui.label(egui::RichText::new("Rotation").color(theme::TEXT_DISABLED).small());
                let rot_changed = ui
                    .horizontal(|ui| {
                        let cx = axis_drag(ui, "X", col_r, &mut deg[0], 0.5);
                        let cy = axis_drag(ui, "Y", col_g, &mut deg[1], 0.5);
                        let cz = axis_drag(ui, "Z", col_b, &mut deg[2], 0.5);
                        cx || cy || cz
                    })
                    .inner;
                if rot_changed {
                    t.rotation = glam::Quat::from_euler(
                        EulerRot::XYZ,
                        deg[0].to_radians(),
                        deg[1].to_radians(),
                        deg[2].to_radians(),
                    );
                }
                ui.label(egui::RichText::new("Scale").color(theme::TEXT_DISABLED).small());
                ui.horizontal(|ui| {
                    axis_drag(ui, "X", col_r, &mut t.scale.x, 0.05);
                    axis_drag(ui, "Y", col_g, &mut t.scale.y, 0.05);
                    axis_drag(ui, "Z", col_b, &mut t.scale.z, 0.05);
                });
            });
        }

        // ---- Light ----
        if self.world.get_light(eid).is_some() {
            let remove_light = component_section(ui, "light", "L", "Light", theme::WARNING, true, |ui| {
                let Some(l) = self.world.get_light_mut(eid) else { return; };
                property_row(ui, "Kind", |ui| {
                    egui::ComboBox::from_id_salt("light_kind")
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
                        if ui.add(egui::Slider::new(&mut inner_deg, 1.0..=89.0)).changed() {
                            l.inner_angle = inner_deg.to_radians();
                            if l.inner_angle > l.outer_angle {
                                l.outer_angle = l.inner_angle;
                            }
                        }
                    });
                    property_row(ui, "Outer", |ui| {
                        if ui.add(egui::Slider::new(&mut outer_deg, 1.0..=89.0)).changed() {
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

        // ---- Camera ----
        if self.world.get_camera(eid).is_some() {
            const CAM_ACCENT: Color32 = Color32::from_rgb(0x89, 0xDC, 0xEB);
            let remove_cam = component_section(ui, "camera", "Cam", "Camera", CAM_ACCENT, true, |ui| {
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
            const LISTENER_ACCENT: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7);
            let remove_al = component_section(ui, "audio_listener", "AL", "AudioListener", LISTENER_ACCENT, true, |ui| {
                let Some(al) = self.world.get_audio_listener_mut(eid) else { return; };
                property_row(ui, "Active", |ui| {
                    ui.checkbox(&mut al.active, "");
                });
                property_row(ui, "Volume", |ui| {
                    ui.add(egui::Slider::new(&mut al.volume, 0.0..=1.0));
                });
            });
            if remove_al {
                self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
                self.world.remove_audio_listener(eid);
            }
        }

        // ---- Canvas ----
        if self.world.get_canvas(eid).is_some() {
            const CANVAS_ACCENT: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7);
            let remove_cv = component_section(ui, "canvas", "Cv", "Canvas", CANVAS_ACCENT, true, |ui| {
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

        // Delegated sections
        self.inspector_material(ui, eid);
        self.inspector_rigid_body(ui, eid);
        self.inspector_collider(ui, eid);
        self.inspector_audio_source(ui, eid);
        self.inspector_ui_element(ui, eid);
        self.inspector_animator(ui, eid);
        self.inspector_skeletal_animator(ui, eid);
        self.inspector_animator_controller(ui, eid);
        self.inspector_bone(ui);
        self.inspector_scripts(ui, eid);
        self.inspector_add_component(ui, eid);

        }); // ScrollArea

        if self.editor_ctx.inspector_editing {
            let pointer_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
            if !pointer_down {
                self.editor_ctx.inspector_editing = false;
            }
        }
    }
}
