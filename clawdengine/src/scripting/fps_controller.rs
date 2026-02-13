use crate::scripting::{GameScript, ScriptContext, KeyCode, MouseButton};
use glam::Vec3;

/// First-person controller: WASD movement, mouse look, click to raycast.
pub struct FPSController {
    speed: f32,
    sensitivity: f32,
    yaw: f32,
    pitch: f32,
    initialized: bool,
}

impl FPSController {
    pub fn new() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
            initialized: false,
        }
    }
}

impl GameScript for FPSController {
    fn name(&self) -> &str { "FPS Controller" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        self.initialized = false;
        if let Some(t) = ctx.get_transform() {
            let forward = t.rotation * Vec3::NEG_Z;
            self.yaw = forward.z.atan2(forward.x);
            self.pitch = forward.y.asin();
        }
        self.initialized = true;
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        if !self.initialized { return; }

        // Mouse look
        let delta = ctx.mouse_delta();
        self.yaw -= delta[0] * self.sensitivity;
        self.pitch = (self.pitch - delta[1] * self.sensitivity).clamp(-1.4, 1.4);

        let rotation = glam::Quat::from_euler(
            glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0,
        );

        // WASD movement (physical keys — AZERTY compatible)
        let mut move_dir = Vec3::ZERO;
        if ctx.is_key_held(KeyCode::KeyW) { move_dir.z -= 1.0; }
        if ctx.is_key_held(KeyCode::KeyS) { move_dir.z += 1.0; }
        if ctx.is_key_held(KeyCode::KeyA) { move_dir.x -= 1.0; }
        if ctx.is_key_held(KeyCode::KeyD) { move_dir.x += 1.0; }
        if ctx.is_key_held(KeyCode::Space) { move_dir.y += 1.0; }
        if ctx.is_key_held(KeyCode::ShiftLeft) { move_dir.y -= 1.0; }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }
        let world_move = rotation * move_dir * self.speed * dt;

        if let Some(t) = ctx.get_transform_mut() {
            t.position += world_move;
            t.rotation = rotation;
        }

        // Left-click: raycast and highlight target
        if ctx.is_mouse_pressed(MouseButton::Left) {
            if let Some(hit) = ctx.raycast_forward(100.0) {
                if hit.entity != ctx.entity {
                    if let Some(mat) = ctx.get_entity_material_mut(hit.entity) {
                        mat.albedo = Vec3::new(1.0, 0.0, 0.0);
                        mat.emission = Vec3::new(0.5, 0.0, 0.0);
                    }
                }
            }
        }

        // Right-click: destroy entity under crosshair
        if ctx.is_mouse_pressed(MouseButton::Right) {
            if let Some(hit) = ctx.raycast_forward(100.0) {
                if hit.entity != ctx.entity {
                    ctx.destroy_entity(hit.entity);
                }
            }
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(egui::DragValue::new(&mut self.speed).speed(0.1).range(1.0..=20.0));
        });
        ui.horizontal(|ui| {
            ui.label("Sensitivity");
            ui.add(egui::DragValue::new(&mut self.sensitivity).speed(0.0005).range(0.001..=0.01));
        });
    }

    fn game_ui(&mut self, ui: &mut egui::Ui) {
        let center = ui.max_rect().center();
        let painter = ui.painter();
        let arm = 10.0;
        let gap = 3.0;
        let stroke = egui::Stroke::new(2.0, egui::Color32::from_white_alpha(220));
        for &(dx, dy) in &[(1.0_f32, 0.0_f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let start = center + egui::vec2(dx * gap, dy * gap);
            let end = center + egui::vec2(dx * (arm + gap), dy * (arm + gap));
            painter.line_segment([start, end], stroke);
        }
        painter.circle_filled(center, 2.0, egui::Color32::from_white_alpha(220));
    }
}
