use egui::{ComboBox, CornerRadius, Stroke};

use crate::editor::context::ComponentKind;
use crate::editor::layout::{EditorTabViewer, component_section};
use crate::editor::theme;

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_scripts(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
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
    }

    pub(super) fn inspector_add_component(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
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
        let has_anim = self.world.get_animator(eid).is_some();
        let has_skel = self.world.get_skeletal_animator(eid).is_some();
        let all_present = has_mat && has_mr && has_rb && has_col && has_cam && has_audio && has_al && has_ui && has_cv && has_anim && has_skel;

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
                    if !has_cam && ui.button("Camera").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::CameraComponent));
                    }
                    if !has_audio && ui.button("AudioSource").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::AudioSource));
                    }
                    if !has_al && ui.button("AudioListener").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::AudioListener));
                    }
                    if !has_ui && ui.button("UiElement").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::UiElement));
                    }
                    if !has_cv && ui.button("Canvas").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::Canvas));
                    }
                    if !has_anim && ui.button("Animator").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::Animator));
                    }
                    if !has_skel && ui.button("SkeletalAnimator").clicked() {
                        self.editor_ctx.pending_add_component = Some((eid, ComponentKind::SkeletalAnimator));
                    }
                }
            });
    }
}
