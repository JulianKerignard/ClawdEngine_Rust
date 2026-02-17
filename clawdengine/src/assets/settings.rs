use serde::{Deserialize, Serialize};

// ---- Sub-structs ----

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RenderingSettings {
    pub msaa_samples: u32,
    pub anisotropy: u32,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for RenderingSettings {
    fn default() -> Self {
        Self {
            msaa_samples: 4,
            anisotropy: 16,
            window_width: 1280,
            window_height: 720,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ShadowSettings {
    pub map_resolution: u32,
    pub light_distance: f32,
    pub ortho_size: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            map_resolution: 2048,
            light_distance: 20.0,
            ortho_size: 15.0,
            near_plane: 0.1,
            far_plane: 50.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LightingSettings {
    pub ambient_color: [f32; 4],
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            ambient_color: [0.12, 0.14, 0.18, 1.0],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct CameraSettings {
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub rotate_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            fov: 45.0,
            near_clip: 0.1,
            far_clip: 100.0,
            rotate_sensitivity: 0.005,
            pan_sensitivity: 0.005,
            zoom_speed: 0.5,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GridSettings {
    pub half_size: i32,
    pub color: [f32; 4],
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            half_size: 10,
            color: [0.4, 0.4, 0.4, 0.5],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PhysicsSettings {
    pub gravity: f32,
    pub ground_y: f32,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: 9.81,
            ground_y: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct BuildSettings {
    pub app_name: String,
    pub bundle_id_prefix: String,
    pub version: String,
    pub min_macos_version: String,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            app_name: String::new(),
            bundle_id_prefix: "com.clawdengine".to_string(),
            version: "1.0.0".to_string(),
            min_macos_version: "13.0".to_string(),
        }
    }
}

// ---- Main struct ----

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ProjectSettings {
    pub rendering: RenderingSettings,
    pub shadows: ShadowSettings,
    pub lighting: LightingSettings,
    pub camera: CameraSettings,
    pub grid: GridSettings,
    pub physics: PhysicsSettings,
    pub build: BuildSettings,
}

// ---- Save / Load ----

pub fn load_settings() -> ProjectSettings {
    let path = "settings.ron";
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(settings) = ron::from_str(&content) {
            return settings;
        }
    }
    ProjectSettings::default()
}

pub fn save_settings(settings: &ProjectSettings) -> anyhow::Result<()> {
    let config = ron::ser::PrettyConfig::default();
    let content = ron::ser::to_string_pretty(settings, config)?;
    std::fs::write("settings.ron", content)?;
    Ok(())
}
