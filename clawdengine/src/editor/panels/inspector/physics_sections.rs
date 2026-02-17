use egui::DragValue;

use crate::editor::layout::{EditorTabViewer, axis_drag, component_section, property_row};
use crate::editor::theme;

const RB_ACCENT: egui::Color32 = egui::Color32::from_rgb(0xFA, 0xB3, 0x87); // peach

impl<'a> EditorTabViewer<'a> {
    pub(super) fn inspector_rigid_body(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_rigid_body(eid).is_none() {
            return;
        }
        let col_r = theme::AXIS_X;
        let col_g = theme::AXIS_Y;
        let col_b = theme::AXIS_Z;
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
            ui.horizontal(|ui| {
                axis_drag(ui, "X", col_r, &mut rb.velocity.x, 0.1);
                axis_drag(ui, "Y", col_g, &mut rb.velocity.y, 0.1);
                axis_drag(ui, "Z", col_b, &mut rb.velocity.z, 0.1);
            });
            ui.label(egui::RichText::new("Angular Vel.").color(theme::TEXT_DISABLED).small());
            ui.horizontal(|ui| {
                axis_drag(ui, "X", col_r, &mut rb.angular_velocity.x, 0.1);
                axis_drag(ui, "Y", col_g, &mut rb.angular_velocity.y, 0.1);
                axis_drag(ui, "Z", col_b, &mut rb.angular_velocity.z, 0.1);
            });
        });
        if remove_rb {
            self.editor_ctx.undo_stack.push(self.world.snapshot(), self.editor_ctx.selected_entities.clone());
            self.world.remove_rigid_body(eid);
        }
    }

    pub(super) fn inspector_collider(&mut self, ui: &mut egui::Ui, eid: crate::core::EntityId) {
        if self.world.get_collider(eid).is_none() {
            return;
        }
        let remove_col = component_section(ui, "collider", "C", "Collider", theme::ACCENT, true, |ui| {
            let Some(col) = self.world.get_collider_mut(eid) else { return; };
            property_row(ui, "Shape", |ui| {
                egui::ComboBox::from_id_salt("collider_shape")
                    .selected_text(match col.shape {
                        crate::core::ColliderShape::Box => "Box",
                        crate::core::ColliderShape::Sphere => "Sphere",
                        crate::core::ColliderShape::Capsule => "Capsule",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut col.shape, crate::core::ColliderShape::Box, "Box");
                        ui.selectable_value(&mut col.shape, crate::core::ColliderShape::Sphere, "Sphere");
                        ui.selectable_value(&mut col.shape, crate::core::ColliderShape::Capsule, "Capsule");
                    });
            });
            property_row(ui, "Center", |ui| {
                axis_drag(ui, "X", theme::AXIS_X, &mut col.center.x, 0.01);
                axis_drag(ui, "Y", theme::AXIS_Y, &mut col.center.y, 0.01);
                axis_drag(ui, "Z", theme::AXIS_Z, &mut col.center.z, 0.01);
            });
            match col.shape {
                crate::core::ColliderShape::Box => {
                    property_row(ui, "Half Extents", |ui| {
                        axis_drag(ui, "X", theme::AXIS_X, &mut col.half_extents.x, 0.01);
                        axis_drag(ui, "Y", theme::AXIS_Y, &mut col.half_extents.y, 0.01);
                        axis_drag(ui, "Z", theme::AXIS_Z, &mut col.half_extents.z, 0.01);
                    });
                }
                crate::core::ColliderShape::Sphere => {
                    property_row(ui, "Radius", |ui| {
                        ui.add(DragValue::new(&mut col.radius).speed(0.01).range(0.01..=100.0));
                    });
                }
                crate::core::ColliderShape::Capsule => {
                    property_row(ui, "Radius", |ui| {
                        ui.add(DragValue::new(&mut col.radius).speed(0.01).range(0.01..=100.0));
                    });
                    property_row(ui, "Height", |ui| {
                        ui.add(DragValue::new(&mut col.height).speed(0.01).range(0.01..=100.0));
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
}
