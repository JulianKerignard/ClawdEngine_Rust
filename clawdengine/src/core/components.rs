use glam::{Quat, Vec3};
use serde::{Serialize, Deserialize};

// ---- Transform ----

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

// ---- MeshRenderer (stub for Story 2.3) ----

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshRenderer {
    pub mesh_id: Option<usize>,
    pub visible: bool,
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self {
            mesh_id: None,
            visible: true,
        }
    }
}

// ---- Material (stub for Story 2.5) ----

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    #[serde(default)]
    pub emission: Vec3,
    #[serde(default)]
    pub texture_path: Option<String>,
    #[serde(skip)]
    pub texture_id: Option<usize>,
    #[serde(default)]
    pub normal_map_path: Option<String>,
    #[serde(skip)]
    pub normal_map_id: Option<usize>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            albedo: Vec3::new(0.8, 0.8, 0.8),
            roughness: 0.5,
            metallic: 0.0,
            emission: Vec3::ZERO,
            texture_path: None,
            texture_id: None,
            normal_map_path: None,
            normal_map_id: None,
        }
    }
}

// ---- Light ----

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LightKind {
    Directional,
    Point,
    Spot,
}

fn default_capsule_height() -> f32 { 1.0 }
fn default_range() -> f32 { 10.0 }
fn default_inner_angle() -> f32 { 30.0_f32.to_radians() }
fn default_outer_angle() -> f32 { 45.0_f32.to_radians() }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    pub intensity: f32,
    #[serde(default = "default_range")]
    pub range: f32,
    #[serde(default = "default_inner_angle")]
    pub inner_angle: f32,
    #[serde(default = "default_outer_angle")]
    pub outer_angle: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Directional,
            color: Vec3::new(1.0, 0.95, 0.85),
            intensity: 1.5,
            range: 10.0,
            inner_angle: 30.0_f32.to_radians(),
            outer_angle: 45.0_f32.to_radians(),
        }
    }
}

// ---- RigidBody (stub for Story 5.1) ----

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RigidBody {
    pub mass: f32,
    pub gravity_enabled: bool,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            mass: 1.0,
            gravity_enabled: true,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        }
    }
}

// ---- Collider ----

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColliderShape {
    Box,
    Sphere,
    Capsule,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Collider {
    pub shape: ColliderShape,
    /// Offset from entity origin (local space).
    pub center: Vec3,
    /// Half-extents for Box shape (local space).
    pub half_extents: Vec3,
    /// Radius for Sphere / Capsule shape (local space).
    pub radius: f32,
    /// Total height for Capsule shape (includes hemispheres).
    #[serde(default = "default_capsule_height")]
    pub height: f32,
    pub restitution: f32,
    pub friction: f32,
    pub is_trigger: bool,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box,
            center: Vec3::ZERO,
            half_extents: Vec3::splat(0.5),
            radius: 0.5,
            height: 1.0,
            restitution: 0.3,
            friction: 0.5,
            is_trigger: false,
        }
    }
}

impl Collider {
    /// Compute world-space AABB from collider params and world transform.
    pub fn world_aabb(&self, position: Vec3, rotation: glam::Quat, scale: Vec3) -> (Vec3, Vec3) {
        let world_center = position + rotation * (self.center * scale);
        match self.shape {
            ColliderShape::Box => {
                let he = self.half_extents * scale;
                // Rotate the 3 half-extent axes and take abs to get world AABB
                let mat = glam::Mat3::from_quat(rotation);
                let wx = (mat.x_axis * he.x).abs();
                let wy = (mat.y_axis * he.y).abs();
                let wz = (mat.z_axis * he.z).abs();
                let world_he = wx + wy + wz;
                (world_center - world_he, world_center + world_he)
            }
            ColliderShape::Sphere => {
                let max_scale = scale.x.max(scale.y).max(scale.z);
                let r = self.radius * max_scale;
                (world_center - Vec3::splat(r), world_center + Vec3::splat(r))
            }
            ColliderShape::Capsule => {
                let max_scale = scale.x.max(scale.y).max(scale.z);
                let r = self.radius * max_scale;
                let half_seg = ((self.height * scale.y) - 2.0 * r).max(0.0) * 0.5;
                // Capsule axis is local Y, rotated
                let axis = rotation * Vec3::Y;
                let top = world_center + axis * half_seg;
                let bot = world_center - axis * half_seg;
                let min = top.min(bot) - Vec3::splat(r);
                let max = top.max(bot) + Vec3::splat(r);
                (min, max)
            }
        }
    }

    /// For Capsule: returns the two endpoints of the inner segment and the world radius.
    pub fn capsule_segment(&self, position: Vec3, rotation: glam::Quat, scale: Vec3) -> (Vec3, Vec3, f32) {
        let world_center = position + rotation * (self.center * scale);
        let max_scale = scale.x.max(scale.y).max(scale.z);
        let r = self.radius * max_scale;
        let half_seg = ((self.height * scale.y) - 2.0 * r).max(0.0) * 0.5;
        let axis = rotation * Vec3::Y;
        (world_center - axis * half_seg, world_center + axis * half_seg, r)
    }
}

// ---- Camera ----

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraComponent {
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub is_main: bool,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            fov_y: 60_f32.to_radians(),
            near: 0.1,
            far: 100.0,
            is_main: true,
        }
    }
}

// ---- AudioSource ----

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioSource {
    pub audio_path: Option<String>,
    pub volume: f32,
    pub pitch: f32,
    pub loop_audio: bool,
    pub play_on_start: bool,
    pub spatial: bool,
    pub max_distance: f32,
    #[serde(skip)]
    pub is_playing: bool,
}

// ---- AudioListener ----

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AudioListener {
    pub active: bool,
    pub volume: f32,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self { active: true, volume: 1.0 }
    }
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            audio_path: None,
            volume: 1.0,
            pitch: 1.0,
            loop_audio: false,
            play_on_start: false,
            spatial: false,
            max_distance: 20.0,
            is_playing: false,
        }
    }
}

// ---- UI Element ----

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiElementKind {
    Text,
    Panel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiElement {
    pub kind: UiElementKind,
    pub text: String,
    pub font_size: f32,
    pub color: Vec3,
    pub alpha: f32,
    pub anchor: UiAnchor,
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub visible: bool,
}

// ---- Canvas ----

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    pub visible: bool,
}

impl Default for Canvas {
    fn default() -> Self {
        Self { width: 1920.0, height: 1080.0, visible: true }
    }
}

impl Default for UiElement {
    fn default() -> Self {
        Self {
            kind: UiElementKind::Text,
            text: "Hello World".to_string(),
            font_size: 16.0,
            color: Vec3::ONE,
            alpha: 1.0,
            anchor: UiAnchor::TopLeft,
            offset: [16.0, 16.0],
            size: [200.0, 40.0],
            visible: true,
        }
    }
}

// ---- Animator (Keyframe Animation) ----

fn default_anim_speed() -> f32 { 1.0 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Quat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<Vec3>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animator {
    pub keyframes: Vec<Keyframe>,
    pub playing: bool,
    pub loop_animation: bool,
    #[serde(default = "default_anim_speed")]
    pub speed: f32,
    #[serde(skip)]
    pub current_time: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            keyframes: Vec::new(),
            playing: false,
            loop_animation: true,
            speed: 1.0,
            current_time: 0.0,
        }
    }
}
