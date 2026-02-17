use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};

// ---- Project Manifest (lives at project root as project.ron) ----

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectManifest {
    pub name: String,
    pub engine_version: String,
    #[serde(default)]
    pub default_scene: Option<String>,
    #[serde(default)]
    pub description: String,
    pub created: String,
}

// ---- Global Registry (lives at ~/.clawdengine/recent_projects.ron) ----

#[derive(Serialize, Deserialize, Clone)]
pub struct RegisteredProject {
    pub path: String,
    pub last_opened: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectRegistry {
    pub recent: Vec<RegisteredProject>,
}

// ---- Registry I/O ----

pub fn registry_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".clawdengine").join("recent_projects.ron")
}

pub fn load_registry() -> ProjectRegistry {
    let path = registry_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(reg) = ron::from_str(&content) {
            return reg;
        }
    }
    ProjectRegistry { recent: Vec::new() }
}

pub fn save_registry(registry: &ProjectRegistry) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = ron::ser::PrettyConfig::default();
    let content = ron::ser::to_string_pretty(registry, config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn register_project(registry: &mut ProjectRegistry, project_path: &str) {
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    if let Some(existing) = registry.recent.iter_mut().find(|r| r.path == project_path) {
        existing.last_opened = now;
    } else {
        registry.recent.push(RegisteredProject {
            path: project_path.to_string(),
            last_opened: now,
        });
    }
}

pub fn unregister_project(registry: &mut ProjectRegistry, project_path: &str) {
    registry.recent.retain(|r| r.path != project_path);
}

// ---- Manifest I/O ----

pub fn load_manifest(project_dir: &str) -> Result<ProjectManifest> {
    let path = Path::new(project_dir).join("project.ron");
    let content = std::fs::read_to_string(&path)?;
    let manifest: ProjectManifest = ron::from_str(&content)?;
    Ok(manifest)
}

pub fn save_manifest(project_dir: &str, manifest: &ProjectManifest) -> Result<()> {
    let path = Path::new(project_dir).join("project.ron");
    let config = ron::ser::PrettyConfig::default();
    let content = ron::ser::to_string_pretty(manifest, config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// ---- Project Creation ----

pub fn create_project(parent_dir: &str, name: &str) -> Result<String> {
    let project_dir = Path::new(parent_dir).join(name);
    std::fs::create_dir_all(project_dir.join("scenes"))?;
    std::fs::create_dir_all(project_dir.join("meshes"))?;
    std::fs::create_dir_all(project_dir.join("textures"))?;
    std::fs::create_dir_all(project_dir.join("audio"))?;
    std::fs::create_dir_all(project_dir.join("animations"))?;

    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let manifest = ProjectManifest {
        name: name.to_string(),
        engine_version: "0.1.0".to_string(),
        default_scene: None,
        description: String::new(),
        created: now,
    };

    let path_str = project_dir.to_string_lossy().to_string();
    save_manifest(&path_str, &manifest)?;
    log::info!("Created project '{}' at {}", name, path_str);
    Ok(path_str)
}

// ---- Scene Utilities ----

pub fn count_entities_in_scene(scene_path: &str) -> Option<usize> {
    let content = std::fs::read_to_string(scene_path).ok()?;
    let scene: super::scene::SceneData = ron::from_str(&content).ok()?;
    Some(scene.entities.len())
}

pub fn count_scenes_in_project(project_dir: &str) -> usize {
    let scenes_dir = Path::new(project_dir).join("scenes");
    if !scenes_dir.exists() {
        return 0;
    }
    std::fs::read_dir(&scenes_dir)
        .map(|dir| {
            dir.flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "ron"))
                .count()
        })
        .unwrap_or(0)
}

pub fn default_projects_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("ClawdEngine Projects")
}

// ---- Game Manifest (embedded in exported .app bundles) ----

#[derive(Serialize, Deserialize, Clone)]
pub struct GameManifest {
    pub name: String,
    pub startup_scene: String,
}

pub fn load_game_manifest() -> Result<GameManifest> {
    // Look for game.ron next to the executable (in .app bundle: Resources/game.ron)
    let path = super::paths::resolve("game.ron");
    let content = std::fs::read_to_string(&path)?;
    let manifest: GameManifest = ron::from_str(&content)?;
    Ok(manifest)
}

/// Format a date string as relative time ("2h ago", "yesterday", "13 Feb")
pub fn format_relative_date(iso: &str) -> String {
    let parsed = chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S");
    let dt = match parsed {
        Ok(dt) => dt,
        Err(_) => return iso.to_string(),
    };

    let now = Local::now().naive_local();
    let diff = now.signed_duration_since(dt);

    if diff.num_minutes() < 1 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_days() == 1 {
        "yesterday".to_string()
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        dt.format("%d %b").to_string()
    }
}
