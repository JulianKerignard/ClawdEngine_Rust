use egui::{Color32, ComboBox, DragValue, Slider};

use crate::editor::layout::{EditorTabViewer, component_section, property_row};

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_ui_element(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_ui_element(eid).is_none() {
            return;
        }
        const UI_ACCENT: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7);
        let remove_ui = component_section(ui, "ui_element", "U", "UiElement", UI_ACCENT, true, |ui| {
            let el = self.world.get_ui_element_mut(eid).unwrap();

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
}
