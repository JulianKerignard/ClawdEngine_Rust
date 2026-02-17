use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

mod animation;
mod app;
mod assets;
mod audio;
mod core;
mod editor;
mod input;
mod physics;
mod renderer;
mod scripting;
mod skeletal_animation;

use renderer::{GpuContext, SceneRenderer};

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    scene: Option<SceneRenderer>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    world: core::World,
    input: input::Input,
    editor_ctx: Option<editor::EditorContext>,
    scripts: Vec<(core::EntityId, Box<dyn scripting::GameScript>)>,
    scripts_started: bool,
    collision_state: physics::collision::CollisionState,
    collision_events: Vec<physics::collision::CollisionEvent>,
    audio: Option<audio::AudioSystem>,
    entity_buf: Vec<core::EntityId>,
    last_frame_time: Instant,
    last_draw_calls: u32,
    last_triangles: u32,
    last_culled: u32,
    log_buffer: editor::console::LogBuffer,
    play_time: f32,
    player_scene: Option<String>,
    player_game_name: Option<String>,
}

impl App {
    fn new(log_buffer: editor::console::LogBuffer) -> Self {
        Self {
            window: None,
            gpu: None,
            scene: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            world: core::World::new(),
            input: input::Input::new(),
            editor_ctx: None,
            scripts: Vec::new(),
            scripts_started: false,
            collision_state: physics::collision::CollisionState::new(),
            collision_events: Vec::new(),
            audio: None,
            entity_buf: Vec::new(),
            last_frame_time: Instant::now(),
            last_draw_calls: 0,
            last_triangles: 0,
            last_culled: 0,
            log_buffer: log_buffer.clone(),
            play_time: 0.0,
            player_scene: None,
            player_game_name: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.init_engine(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }
}

impl App {
    fn handle_redraw(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        if let Some(ec) = &mut self.editor_ctx {
            let new_fps = 1.0 / dt.max(0.001);
            ec.fps = ec.fps * 0.95 + new_fps * 0.05;
            ec.entity_count = self.world.entity_count();
            ec.draw_calls = self.last_draw_calls;
            ec.visible_triangles = self.last_triangles;
            ec.culled_entities = self.last_culled;
        }

        let is_hub = self.editor_ctx.as_ref()
            .is_some_and(|ec| ec.screen == editor::context::AppScreen::Hub);

        if !is_hub {
            let wants_kb = self.egui_ctx.wants_keyboard_input();
            let right_held = self.input.is_mouse_held(MouseButton::Right);
            let focus_requested = if let Some(ec) = &mut self.editor_ctx {
                editor::shortcuts::handle_shortcuts(&self.input, ec, wants_kb, right_held)
            } else {
                false
            };

            self.update_camera(dt, focus_requested);
            self.update_gizmo();
            self.run_game_systems(dt);
        }

        // Resize viewport texture if dock panel changed size
        if let (Some(gpu), Some(scene), Some(ec)) =
            (&mut self.gpu, &mut self.scene, &self.editor_ctx)
        {
            let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32);

            let vp = ec.viewport_rect;
            let new_w = (vp.width() * scale).max(1.0) as u32;
            let new_h = (vp.height() * scale).max(1.0) as u32;
            if new_w != scene.viewport.width || new_h != scene.viewport.height {
                scene.viewport.resize(&gpu.device, &mut gpu.egui_renderer, new_w, new_h);
            }

            let gvp = ec.game_viewport_rect;
            let gw = (gvp.width() * scale).max(1.0) as u32;
            let gh = (gvp.height() * scale).max(1.0) as u32;
            if let Some(game_vp) = &mut scene.game_viewport {
                if gw != game_vp.width || gh != game_vp.height {
                    game_vp.resize(&gpu.device, &mut gpu.egui_renderer, gw, gh);
                }
            }
        }

        let (frame_dc, frame_tri, frame_culled) = self.render_frame();
        self.last_draw_calls = frame_dc;
        self.last_triangles = frame_tri;
        self.last_culled = frame_culled;

        // Process pending entity operations
        if let (Some(editor_ctx), Some(scene), Some(gpu)) =
            (&mut self.editor_ctx, &mut self.scene, &self.gpu)
        {
            editor::pending_ops::process_pending_operations(
                &mut self.world,
                editor_ctx,
                scene,
                gpu,
                &mut self.scripts,
                &mut self.scripts_started,
                dt,
            );
        }

        // Clear collision state when exiting play mode
        if !self.editor_ctx.as_ref().is_some_and(|ec| ec.play_mode) && !self.collision_events.is_empty() {
            self.collision_events.clear();
            self.collision_state.clear();
        }

        self.input.begin_frame();

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn render_frame(&mut self) -> (u32, u32, u32) {
        let selected = self.editor_ctx.as_ref().and_then(|ec| ec.primary_selection());
        let gizmo_pos = self.editor_ctx.as_ref().and_then(|ec| ec.selection_center(&self.world));

        let (Some(gpu), Some(window), Some(egui_state), Some(scene)) = (
            &mut self.gpu,
            &self.window,
            &mut self.egui_state,
            &mut self.scene,
        ) else {
            return (0, 0, 0);
        };

        let game_view_was_visible = self.editor_ctx.as_ref()
            .is_some_and(|ec| ec.game_view_visible);
        let game_vp_tex = if game_view_was_visible {
            Some(scene.ensure_game_viewport(&gpu.device, &mut gpu.egui_renderer))
        } else {
            scene.game_viewport.as_ref().map(|gv| gv.egui_texture_id)
        };

        if let Some(ec) = &mut self.editor_ctx {
            ec.game_view_visible = false;
        }

        let viewport_tex_id = scene.viewport.egui_texture_id;
        let egui_ctx = self.egui_ctx.clone();
        let active_tool = self.editor_ctx.as_ref()
            .map(|ec| ec.active_tool)
            .unwrap_or(editor::context::EditorTool::Select);
        let gizmo_drag_axis = self.editor_ctx.as_ref().and_then(|ec| {
            ec.gizmo_drag.as_ref().map(|d| match d {
                editor::context::GizmoDragState::Move { axis, .. } => *axis,
                editor::context::GizmoDragState::Rotate { axis, .. } => *axis,
                editor::context::GizmoDragState::Scale { axis, .. } => *axis,
            })
        });
        let gizmo_hover_axis = self.editor_ctx.as_ref()
            .and_then(|ec| ec.hovered_gizmo_axis);
        let editor_ctx = &mut self.editor_ctx;
        let scripts = &mut self.scripts;
        let is_hub = editor_ctx.as_ref()
            .is_some_and(|ec| ec.screen == editor::context::AppScreen::Hub);
        let show_grid = if is_hub { false } else { editor_ctx.as_ref().is_none_or(|ec| ec.show_grid) };
        let render_game = if is_hub { false } else { game_view_was_visible };

        let settings = editor_ctx.as_ref()
            .map(|ec| ec.project_settings.clone())
            .unwrap_or_default();

        gpu.render_frame(
            &egui_ctx,
            egui_state,
            window,
            scene,
            &mut self.world,
            selected,
            gizmo_pos,
            active_tool,
            gizmo_drag_axis,
            gizmo_hover_axis,
            show_grid,
            render_game,
            &settings,
            |ctx, world, scene_ref| {
                if let Some(ec) = editor_ctx {
                    match ec.screen {
                        editor::context::AppScreen::Hub => {
                            editor::project_hub::show(ctx, ec);
                        }
                        editor::context::AppScreen::Editor => {
                            editor::EditorLayout::show(
                                ctx,
                                Some(viewport_tex_id),
                                game_vp_tex,
                                world,
                                ec,
                                scripts,
                                scene_ref,
                            );
                        }
                    }
                }
            },
        )
    }
}

fn main() {
    let log_buffer = editor::console::init_logger();
    log::info!("ClawdEngine starting...");

    let args: Vec<String> = std::env::args().collect();
    let mut player_scene = None;
    let mut player_name = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--player" if i + 1 < args.len() => {
                player_scene = Some(args[i + 1].clone());
                i += 2;
            }
            "--name" if i + 1 < args.len() => {
                player_name = Some(args[i + 1].clone());
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new(log_buffer);
    app.player_scene = player_scene;
    app.player_game_name = player_name;
    event_loop.run_app(&mut app).expect("Event loop error");
}
