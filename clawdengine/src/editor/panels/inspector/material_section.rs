use egui::Color32;

use crate::editor::layout::{
    EditorTabViewer, TextureSlotAction, component_section, property_row, texture_slot,
};
use crate::editor::theme;

const MAT_ACCENT: Color32 = Color32::from_rgb(0xF5, 0xC2, 0xE7); // pink

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_material(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_material(eid).is_none() {
            return;
        }
        let remove_mat = component_section(ui, "material", "M", "Material", MAT_ACCENT, true, |ui| {
            let Some(m) = self.world.get_material_mut(eid) else { return; };

            ui.label(egui::RichText::new("Surface").color(theme::TEXT_DISABLED).small());
            property_row(ui, "Color", |ui| {
                let mut color = [m.albedo.x, m.albedo.y, m.albedo.z];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    m.albedo = glam::Vec3::new(color[0], color[1], color[2]);
                }
            });
            property_row(ui, "Roughness", |ui| {
                ui.add(egui::Slider::new(&mut m.roughness, 0.0..=1.0).show_value(true));
            });
            property_row(ui, "Metallic", |ui| {
                ui.add(egui::Slider::new(&mut m.metallic, 0.0..=1.0).show_value(true));
            });

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Emission").color(theme::TEXT_DISABLED).small());
            property_row(ui, "Color", |ui| {
                let mut em = [m.emission.x, m.emission.y, m.emission.z];
                if ui.color_edit_button_rgb(&mut em).changed() {
                    m.emission = glam::Vec3::new(em[0], em[1], em[2]);
                }
            });

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Textures").color(theme::TEXT_DISABLED).small());

            let albedo_action = texture_slot(
                ui, "Albedo", &m.texture_path, Color32::from_rgb(0x89, 0xB4, 0xFA),
            );
            if albedo_action == TextureSlotAction::Remove {
                m.texture_path = None;
                m.texture_id = None;
            }

            let albedo_btn = ui.small_button("Browse Texture...");
            egui::Popup::from_toggle_button_response(&albedo_btn)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui: &mut egui::Ui| {
                    ui.set_min_width(200.0);
                    Self::texture_browser(ui, eid, false, &mut self.editor_ctx.pending_texture_assign);
                });

            ui.add_space(2.0);

            let normal_action = texture_slot(
                ui, "Normal Map", &m.normal_map_path, Color32::from_rgb(0x94, 0xE2, 0xD5),
            );
            if normal_action == TextureSlotAction::Remove {
                m.normal_map_path = None;
                m.normal_map_id = None;
            }

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

    fn texture_browser(
        ui: &mut egui::Ui,
        eid: crate::core::EntityId,
        _is_normal: bool,
        target: &mut Option<(crate::core::EntityId, String)>,
    ) {
        let mut found_any = false;
        if let Ok(entries) = std::fs::read_dir(crate::assets::paths::resolve("assets/textures")) {
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
