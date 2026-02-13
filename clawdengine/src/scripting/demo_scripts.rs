use crate::scripting::{GameScript, ScriptContext};
use egui::DragValue;

// ---- Helper ----

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> glam::Vec3 {
    let c = v * s;
    let h6 = h * 6.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    glam::Vec3::new(r + m, g + m, b + m)
}

// ---- Demo Scripts ----

/// Rotates entity on Y axis and pulses color between red and blue.
pub struct RotateAndPulse {
    time: f32,
    pub speed: f32,
}

impl RotateAndPulse {
    pub fn new() -> Self {
        Self { time: 0.0, speed: 1.0 }
    }
}

impl GameScript for RotateAndPulse {
    fn name(&self) -> &str { "Rotate & Pulse" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        self.time = 0.0;
        if let Some(rb) = ctx.get_rigid_body_mut() {
            rb.gravity_enabled = false;
        }
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.time += dt * self.speed;

        if let Some(t) = ctx.get_transform_mut() {
            t.rotation = glam::Quat::from_rotation_y(
                self.time * std::f32::consts::FRAC_PI_2,
            );
        }

        if let Some(m) = ctx.get_material_mut() {
            let s = (self.time * 2.0).sin() * 0.5 + 0.5;
            m.albedo = glam::Vec3::new(s, 0.2, 1.0 - s);
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(DragValue::new(&mut self.speed).speed(0.05).range(0.0..=10.0));
        });
    }
}

/// Enables gravity with an upward impulse, then bounces on ground with damping.
pub struct GravityBounce {
    bounce_factor: f32,
}

impl GravityBounce {
    pub fn new() -> Self {
        Self { bounce_factor: 0.6 }
    }
}

impl GameScript for GravityBounce {
    fn name(&self) -> &str { "Gravity Bounce" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        if let Some(rb) = ctx.get_rigid_body_mut() {
            rb.gravity_enabled = true;
            rb.velocity.y = 5.0;
        }
    }

    fn update(&mut self, ctx: &mut ScriptContext, _dt: f32) {
        let should_bounce = ctx.get_transform()
            .is_some_and(|t| t.position.y <= 0.01);
        let bounce_vel = if should_bounce {
            ctx.get_rigid_body()
                .filter(|rb| rb.velocity.y <= 0.0)
                .map(|rb| (-rb.velocity.y * self.bounce_factor).max(0.0))
        } else {
            None
        };
        if let Some(vel) = bounce_vel {
            if let Some(rb) = ctx.get_rigid_body_mut() {
                rb.velocity.y = vel;
            }
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Bounce");
            ui.add(egui::Slider::new(&mut self.bounce_factor, 0.0..=1.0));
        });
    }
}

/// Cycles material color through the HSV spectrum.
pub struct ColorCycle {
    time: f32,
    pub speed: f32,
}

impl ColorCycle {
    pub fn new() -> Self {
        Self { time: 0.0, speed: 0.3 }
    }
}

impl GameScript for ColorCycle {
    fn name(&self) -> &str { "Color Cycle" }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.time += dt;
        let hue = (self.time * self.speed) % 1.0;
        let rgb = hsv_to_rgb(hue, 0.8, 0.9);
        if let Some(m) = ctx.get_material_mut() {
            m.albedo = rgb;
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(DragValue::new(&mut self.speed).speed(0.01).range(0.01..=5.0));
        });
    }
}

// ---- User-created configurable script ----

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptBehavior {
    None,
    Rotate,
    Bounce,
    Oscillate,
    ColorCycle,
}

const BEHAVIORS: [ScriptBehavior; 5] = [
    ScriptBehavior::None,
    ScriptBehavior::Rotate,
    ScriptBehavior::Bounce,
    ScriptBehavior::Oscillate,
    ScriptBehavior::ColorCycle,
];

pub struct UserScript {
    display_name: String,
    behavior: ScriptBehavior,
    speed: f32,
    time: f32,
    base_y: f32,
    bounce_factor: f32,
}

impl UserScript {
    pub fn new(name: String) -> Self {
        Self {
            display_name: name,
            behavior: ScriptBehavior::None,
            speed: 1.0,
            time: 0.0,
            base_y: 0.0,
            bounce_factor: 0.6,
        }
    }
}

impl GameScript for UserScript {
    fn name(&self) -> &str { &self.display_name }

    fn start(&mut self, ctx: &mut ScriptContext) {
        self.time = 0.0;
        match self.behavior {
            ScriptBehavior::Rotate => {
                if let Some(rb) = ctx.get_rigid_body_mut() {
                    rb.gravity_enabled = false;
                }
            }
            ScriptBehavior::Bounce => {
                if let Some(rb) = ctx.get_rigid_body_mut() {
                    rb.gravity_enabled = true;
                    rb.velocity.y = 5.0;
                }
            }
            ScriptBehavior::Oscillate => {
                if let Some(t) = ctx.get_transform() {
                    self.base_y = t.position.y;
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.time += dt;
        match self.behavior {
            ScriptBehavior::None => {}
            ScriptBehavior::Rotate => {
                let t_scaled = self.time * self.speed;
                if let Some(t) = ctx.get_transform_mut() {
                    t.rotation = glam::Quat::from_rotation_y(
                        t_scaled * std::f32::consts::FRAC_PI_2,
                    );
                }
                if let Some(m) = ctx.get_material_mut() {
                    let s = (t_scaled * 2.0).sin() * 0.5 + 0.5;
                    m.albedo = glam::Vec3::new(s, 0.2, 1.0 - s);
                }
            }
            ScriptBehavior::Bounce => {
                let should_bounce = ctx.get_transform()
                    .is_some_and(|t| t.position.y <= 0.01);
                let bounce_vel = if should_bounce {
                    ctx.get_rigid_body()
                        .filter(|rb| rb.velocity.y <= 0.0)
                        .map(|rb| (-rb.velocity.y * self.bounce_factor).max(0.0))
                } else {
                    None
                };
                if let Some(vel) = bounce_vel {
                    if let Some(rb) = ctx.get_rigid_body_mut() {
                        rb.velocity.y = vel;
                    }
                }
            }
            ScriptBehavior::Oscillate => {
                if let Some(t) = ctx.get_transform_mut() {
                    t.position.y = self.base_y
                        + (self.time * std::f32::consts::TAU * self.speed).sin() * 1.5;
                }
            }
            ScriptBehavior::ColorCycle => {
                let hue = (self.time * self.speed) % 1.0;
                let rgb = hsv_to_rgb(hue, 0.8, 0.9);
                if let Some(m) = ctx.get_material_mut() {
                    m.albedo = rgb;
                }
            }
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Behavior");
            egui::ComboBox::from_id_salt("behavior_select")
                .selected_text(format!("{:?}", self.behavior))
                .show_ui(ui, |ui| {
                    for b in &BEHAVIORS {
                        ui.selectable_value(&mut self.behavior, *b, format!("{:?}", b));
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(DragValue::new(&mut self.speed).speed(0.05).range(0.0..=10.0));
        });
        if self.behavior == ScriptBehavior::Bounce {
            ui.horizontal(|ui| {
                ui.label("Bounce");
                ui.add(egui::Slider::new(&mut self.bounce_factor, 0.0..=1.0));
            });
        }
    }
}

/// Oscillates entity up and down on Y axis with a sine wave.
pub struct Oscillate {
    time: f32,
    base_y: f32,
    pub amplitude: f32,
    pub frequency: f32,
}

impl Oscillate {
    pub fn new() -> Self {
        Self { time: 0.0, base_y: 0.0, amplitude: 1.5, frequency: 1.0 }
    }
}

impl GameScript for Oscillate {
    fn name(&self) -> &str { "Oscillate" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        self.time = 0.0;
        if let Some(t) = ctx.get_transform() {
            self.base_y = t.position.y;
        }
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.time += dt;
        if let Some(t) = ctx.get_transform_mut() {
            t.position.y = self.base_y
                + (self.time * std::f32::consts::TAU * self.frequency).sin() * self.amplitude;
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Amplitude");
            ui.add(DragValue::new(&mut self.amplitude).speed(0.05).range(0.1..=10.0));
        });
        ui.horizontal(|ui| {
            ui.label("Frequency");
            ui.add(DragValue::new(&mut self.frequency).speed(0.05).range(0.1..=5.0));
        });
    }
}

/// Demo script: plays audio on start, then replays periodically.
pub struct AudioDemo {
    time: f32,
    interval: f32,
    #[allow(dead_code)]
    started: bool,
}

impl AudioDemo {
    pub fn new() -> Self {
        Self { time: 0.0, interval: 3.0, started: false }
    }
}

impl GameScript for AudioDemo {
    fn name(&self) -> &str { "Audio Demo" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        self.time = 0.0;
        self.started = false;
        ctx.play_audio();
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.time += dt;
        if self.time >= self.interval {
            self.time = 0.0;
            ctx.play_audio();
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Interval");
            ui.add(DragValue::new(&mut self.interval).speed(0.1).range(0.5..=30.0));
        });
    }
}

// ---- Player HUD (demo) ----

pub struct PlayerHUD {
    health: f32,
    max_health: f32,
    score: u32,
    time: f32,
}

impl PlayerHUD {
    pub fn new() -> Self {
        Self { health: 80.0, max_health: 100.0, score: 0, time: 0.0 }
    }
}

impl GameScript for PlayerHUD {
    fn name(&self) -> &str { "Player HUD" }

    fn start(&mut self, _ctx: &mut ScriptContext) {
        self.health = 80.0;
        self.score = 0;
        self.time = 0.0;
    }

    fn update(&mut self, _ctx: &mut ScriptContext, dt: f32) {
        self.time += dt;
        self.score = (self.time * 10.0) as u32;
        self.health = (self.health + dt * 2.0).min(self.max_health);
        if (self.time % 5.0) < dt {
            self.health = (self.health - 15.0).max(0.0);
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Health");
            ui.add(egui::Slider::new(&mut self.health, 0.0..=self.max_health));
        });
        ui.horizontal(|ui| {
            ui.label("Score");
            ui.add(DragValue::new(&mut self.score));
        });
    }

    fn game_ui(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();

        // ---- Health bar (top-left) ----
        let bar_pos = rect.left_top() + egui::vec2(16.0, 16.0);
        let bar_size = egui::vec2(200.0, 20.0);
        let bar_rect = egui::Rect::from_min_size(bar_pos, bar_size);

        painter.rect_filled(bar_rect, 4.0, egui::Color32::from_black_alpha(160));
        let fill_frac = (self.health / self.max_health).clamp(0.0, 1.0);
        let fill_color = if fill_frac > 0.5 {
            egui::Color32::from_rgb(0x40, 0xC0, 0x40)
        } else if fill_frac > 0.25 {
            egui::Color32::from_rgb(0xE0, 0xA0, 0x20)
        } else {
            egui::Color32::from_rgb(0xE0, 0x30, 0x30)
        };
        let fill_rect = egui::Rect::from_min_size(
            bar_pos,
            egui::vec2(bar_size.x * fill_frac, bar_size.y),
        );
        painter.rect_filled(fill_rect, 4.0, fill_color);
        painter.rect_stroke(bar_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
        painter.text(
            bar_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.0} / {:.0}", self.health, self.max_health),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );

        // ---- Score (top-right) ----
        painter.text(
            egui::pos2(rect.right() - 16.0, rect.top() + 24.0),
            egui::Align2::RIGHT_TOP,
            format!("Score: {}", self.score),
            egui::FontId::monospace(18.0),
            egui::Color32::from_rgb(0xFF, 0xD7, 0x00),
        );

        // ---- Crosshair (center) ----
        let center = rect.center();
        let gap = 4.0;
        let arm = 12.0;
        let stroke = egui::Stroke::new(2.0, egui::Color32::from_white_alpha(200));
        painter.line_segment(
            [center - egui::vec2(arm + gap, 0.0), center - egui::vec2(gap, 0.0)],
            stroke,
        );
        painter.line_segment(
            [center + egui::vec2(gap, 0.0), center + egui::vec2(arm + gap, 0.0)],
            stroke,
        );
        painter.line_segment(
            [center - egui::vec2(0.0, arm + gap), center - egui::vec2(0.0, gap)],
            stroke,
        );
        painter.line_segment(
            [center + egui::vec2(0.0, gap), center + egui::vec2(0.0, arm + gap)],
            stroke,
        );
        painter.circle_filled(center, 2.0, egui::Color32::from_white_alpha(200));
    }
}
