use std::path::PathBuf;

use glam::{Quat, Vec3};

use crate::core::EntityId;
use crate::core::world::WorldSnapshot;
use crate::scripting::GameScript;
use super::icons::EditorIcons;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppScreen {
    Hub,
    Editor,
}

#[derive(Clone, Debug)]
pub enum HubAction {
    NewBlank,
    NewDemo,
    OpenScene(String),
}

#[derive(Clone, Debug)]
pub enum AssetEntry {
    Folder(String),
    File(String),
}

#[derive(Clone, Debug)]
pub enum AssetModal {
    NewFolder { name: String },
    NewScript { name: String },
    NewScene { name: String },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

impl GizmoAxis {
    pub fn direction(self) -> Vec3 {
        match self {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
        }
    }
}

pub enum GizmoDragState {
    Move {
        axis: GizmoAxis,
        initial_t: f32,
        center: Vec3,
        initial_positions: Vec<(EntityId, Vec3)>,
    },
    Rotate {
        axis: GizmoAxis,
        initial_angle: f32,
        center: Vec3,
        initial_transforms: Vec<(EntityId, Vec3, Quat)>,
    },
    Scale {
        axis: GizmoAxis,
        initial_t: f32,
        center: Vec3,
        initial_scales: Vec<(EntityId, Vec3, Vec3)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Clone, Debug)]
pub enum SpawnRequest {
    Empty,
    Cube,
    Sphere,
    Light,
    Camera,
    Audio,
    Canvas,
    UiText,
    UiPanel,
}

#[derive(Clone, Debug)]
pub enum ComponentKind {
    Material,
    MeshRenderer,
    RigidBody,
    Collider,
    CameraComponent,
    AudioSource,
    AudioListener,
    UiElement,
    Canvas,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorTab {
    Hierarchy,
    Viewport,
    Inspector,
    Assets,
    Console,
    GameView,
}

pub struct BuiltinMeshes {
    pub cube: usize,
    pub sphere: usize,
}

pub struct UndoEntry {
    pub snapshot: WorldSnapshot,
    pub selected: Vec<EntityId>,
}

const UNDO_MAX: usize = 32;

pub struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self { undo: Vec::new(), redo: Vec::new() }
    }

    pub fn push(&mut self, snapshot: WorldSnapshot, selected: Vec<EntityId>) {
        self.redo.clear();
        if self.undo.len() >= UNDO_MAX {
            self.undo.remove(0);
        }
        self.undo.push(UndoEntry { snapshot, selected });
    }

    pub fn pop(&mut self) -> Option<UndoEntry> {
        self.undo.pop()
    }

    pub fn push_redo(&mut self, snapshot: WorldSnapshot, selected: Vec<EntityId>) {
        if self.redo.len() >= UNDO_MAX {
            self.redo.remove(0);
        }
        self.redo.push(UndoEntry { snapshot, selected });
    }

    pub fn pop_redo(&mut self) -> Option<UndoEntry> {
        self.redo.pop()
    }

    /// Push into undo stack WITHOUT clearing redo (used by process_redo)
    pub fn push_undo_only(&mut self, snapshot: WorldSnapshot, selected: Vec<EntityId>) {
        if self.undo.len() >= UNDO_MAX {
            self.undo.remove(0);
        }
        self.undo.push(UndoEntry { snapshot, selected });
    }

    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
}

pub struct ScriptRegistryEntry {
    pub name: String,
    pub factory: Box<dyn Fn() -> Box<dyn GameScript>>,
}

pub struct EditorContext {
    pub selected_entities: Vec<EntityId>,
    pub pending_spawn: Option<SpawnRequest>,
    pub pending_delete: Vec<EntityId>,
    pub pending_duplicate: Vec<EntityId>,
    pub builtin_meshes: BuiltinMeshes,
    pub show_add_menu: bool,
    pub viewport_rect: egui::Rect,
    pub play_mode: bool,
    pub pending_play_toggle: Option<bool>,
    pub snapshot: Option<WorldSnapshot>,
    pub fps: f32,
    pub entity_count: usize,
    pub draw_calls: u32,
    pub visible_triangles: u32,
    pub show_stats_overlay: bool,
    pub show_grid: bool,
    pub dock_state: egui_dock::DockState<EditorTab>,
    pub gizmo_drag: Option<GizmoDragState>,
    pub active_tool: EditorTool,
    pub undo_stack: UndoStack,
    pub pending_undo: bool,
    pub pending_redo: bool,
    /// Asset browser entries (folders + files)
    pub asset_entries: Vec<AssetEntry>,
    /// Current directory in the asset browser
    pub asset_current_dir: PathBuf,
    /// Modal dialog for creating folder/script
    pub asset_modal: Option<AssetModal>,
    /// Pending folder creation
    pub pending_create_folder: Option<String>,
    /// Pending script creation
    pub pending_create_script: Option<String>,
    /// Pending asset deletion (file or folder path)
    pub pending_delete_asset: Option<PathBuf>,
    /// Script registry: available script types
    pub script_registry: Vec<ScriptRegistryEntry>,
    /// Pending script attach: (entity, registry_index)
    pub pending_add_script: Option<(EntityId, usize)>,
    /// Pending asset load: filename from assets/meshes/
    pub pending_load_asset: Option<String>,
    /// Pending script removals: (entity_id, script_name)
    pub pending_remove_scripts: Vec<(EntityId, String)>,
    /// Hovered gizmo axis (for visual highlight)
    pub hovered_gizmo_axis: Option<GizmoAxis>,
    /// Hierarchy panel: expanded entities (with children)
    pub hierarchy_expanded: std::collections::HashSet<EntityId>,
    /// Hierarchy drag source (entity being dragged)
    pub hierarchy_drag_source: Option<EntityId>,
    /// Pending reparent: (child, new_parent or None to detach)
    pub pending_reparent: Option<(EntityId, Option<EntityId>)>,
    /// Engine icons (loaded lazily on first egui frame)
    pub icons: Option<EditorIcons>,
    /// Pending new scene with name
    pub pending_new_scene_name: Option<String>,
    /// Pending scene save (scene name)
    pub pending_save_scene: Option<String>,
    /// Pending scene load (scene name)
    pub pending_load_scene: Option<String>,
    /// Current scene name
    pub scene_name: String,
    /// Save/load feedback notification (message, remaining seconds)
    pub save_feedback: Option<(String, f32)>,
    /// Pending component addition: (entity, component kind)
    pub pending_add_component: Option<(EntityId, ComponentKind)>,
    /// Pending texture assign: (entity, texture_path)
    pub pending_texture_assign: Option<(EntityId, String)>,
    /// Pending normal map assign: (entity, texture_path)
    pub pending_normal_map_assign: Option<(EntityId, String)>,
    /// Async glTF loading state machine
    pub asset_load_state: Option<super::asset_loader::AssetLoadState>,
    /// Loading status message for UI feedback
    pub loading_status: Option<String>,
    /// Viewport-relative cursor position at drop time (sx, sy)
    pub pending_drop_screen_pos: Option<(f32, f32)>,
    /// Shared log buffer for console panel
    pub log_buffer: super::console::LogBuffer,
    pub console_filter_info: bool,
    pub console_filter_warn: bool,
    pub console_filter_error: bool,
    pub console_filter_debug: bool,
    pub console_auto_scroll: bool,
    pub game_view_visible: bool,
    pub game_viewport_rect: egui::Rect,
    /// True while the user is dragging/editing values in the inspector
    pub inspector_editing: bool,
    /// Hierarchy search query
    pub hierarchy_search: String,
    /// Asset browser search query
    pub asset_search: String,
    /// Fullscreen game mode (all editor panels hidden, game fills window)
    pub fullscreen_game: bool,
    /// Current screen: Hub (welcome) or Editor
    pub screen: AppScreen,
    /// Action selected in the hub (processed next frame)
    pub pending_hub_action: Option<HubAction>,
    /// Cached thumbnail textures for the project hub
    pub hub_thumbnails: std::collections::HashMap<String, egui::TextureHandle>,
    /// Rename modal state: (original_name, text_input)
    pub hub_rename: Option<(String, String)>,
}

impl EditorContext {
    pub fn new(cube_mesh_id: usize, sphere_mesh_id: usize, log_buffer: super::console::LogBuffer) -> Self {
        Self {
            selected_entities: Vec::new(),
            pending_spawn: None,
            pending_delete: Vec::new(),
            pending_duplicate: Vec::new(),
            builtin_meshes: BuiltinMeshes {
                cube: cube_mesh_id,
                sphere: sphere_mesh_id,
            },
            show_add_menu: false,
            viewport_rect: egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(800.0, 600.0),
            ),
            play_mode: false,
            pending_play_toggle: None,
            snapshot: None,
            fps: 0.0,
            entity_count: 0,
            draw_calls: 0,
            visible_triangles: 0,
            show_stats_overlay: true,
            show_grid: true,
            dock_state: Self::default_dock_state(),
            gizmo_drag: None,
            active_tool: EditorTool::Move,
            undo_stack: UndoStack::new(),
            pending_undo: false,
            pending_redo: false,
            asset_entries: Vec::new(),
            asset_current_dir: PathBuf::from("assets"),
            asset_modal: None,
            pending_create_folder: None,
            pending_create_script: None,
            pending_delete_asset: None,
            script_registry: Vec::new(),
            pending_add_script: None,
            pending_load_asset: None,
            pending_remove_scripts: Vec::new(),
            hovered_gizmo_axis: None,
            hierarchy_expanded: std::collections::HashSet::new(),
            hierarchy_drag_source: None,
            pending_reparent: None,
            icons: None,
            pending_new_scene_name: None,
            pending_save_scene: None,
            pending_load_scene: None,
            scene_name: "scene".to_string(),
            save_feedback: None,
            pending_add_component: None,
            pending_texture_assign: None,
            pending_normal_map_assign: None,
            asset_load_state: None,
            loading_status: None,
            pending_drop_screen_pos: None,
            log_buffer,
            console_filter_info: true,
            console_filter_warn: true,
            console_filter_error: true,
            console_filter_debug: false,
            console_auto_scroll: true,
            game_view_visible: false,
            game_viewport_rect: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(800.0, 600.0)),
            inspector_editing: false,
            hierarchy_search: String::new(),
            asset_search: String::new(),
            fullscreen_game: false,
            screen: AppScreen::Hub,
            pending_hub_action: None,
            hub_thumbnails: std::collections::HashMap::new(),
            hub_rename: None,
        }
    }

    /// Creates the default dock layout: Hierarchy (15%) | Viewport (60%) | Inspector (25%)
    pub fn default_dock_state() -> egui_dock::DockState<EditorTab> {
        let mut dock_state = egui_dock::DockState::new(vec![EditorTab::Viewport, EditorTab::GameView]);
        let surface = dock_state.main_surface_mut();
        let [_old, _left] = surface.split_left(
            egui_dock::NodeIndex::root(),
            0.15,
            vec![EditorTab::Hierarchy],
        );
        // After split_left(0.15), root is 85% right. 25/85 ≈ 0.294
        let [_center, _right] = surface.split_right(
            egui_dock::NodeIndex::root(),
            0.294,
            vec![EditorTab::Inspector],
        );
        // Assets + Console tabs below Hierarchy (left column)
        let [_top, _bottom] = surface.split_below(
            _left,
            0.6,
            vec![EditorTab::Assets, EditorTab::Console],
        );
        dock_state
    }

    pub fn select(&mut self, id: EntityId) {
        self.selected_entities = vec![id];
    }

    pub fn toggle_select(&mut self, id: EntityId) {
        if let Some(pos) = self.selected_entities.iter().position(|&e| e == id) {
            self.selected_entities.remove(pos);
        } else {
            self.selected_entities.push(id);
        }
    }

    pub fn deselect_all(&mut self) {
        self.selected_entities.clear();
    }

    pub fn primary_selection(&self) -> Option<EntityId> {
        self.selected_entities.first().copied()
    }

    pub fn is_selected(&self, id: EntityId) -> bool {
        self.selected_entities.contains(&id)
    }

    pub fn selection_center(&self, world: &crate::core::World) -> Option<Vec3> {
        if self.selected_entities.is_empty() {
            return None;
        }
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for &eid in &self.selected_entities {
            if let Some(wt) = world.get_world_transform(eid) {
                sum += wt.position;
                count += 1;
            }
        }
        if count > 0 { Some(sum / count as f32) } else { None }
    }

    pub fn refresh_assets(&mut self) {
        self.asset_entries.clear();
        if let Ok(entries) = std::fs::read_dir(&self.asset_current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy().into_owned();
                    if name_str.starts_with('.') {
                        continue;
                    }
                    if path.is_dir() {
                        self.asset_entries.push(AssetEntry::Folder(name_str));
                    } else {
                        self.asset_entries.push(AssetEntry::File(name_str));
                    }
                }
            }
        }
        // Folders first, then files, each sorted alphabetically
        self.asset_entries.sort_by(|a, b| match (a, b) {
            (AssetEntry::Folder(_), AssetEntry::File(_)) => std::cmp::Ordering::Less,
            (AssetEntry::File(_), AssetEntry::Folder(_)) => std::cmp::Ordering::Greater,
            (AssetEntry::Folder(a), AssetEntry::Folder(b)) => a.cmp(b),
            (AssetEntry::File(a), AssetEntry::File(b)) => a.cmp(b),
        });
    }
}
