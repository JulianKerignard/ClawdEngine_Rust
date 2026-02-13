use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

mod assets;
mod audio;
mod core;
mod editor;
mod input;
mod physics;
mod renderer;
mod scripting;

use editor::context::ScriptRegistryEntry;
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
    last_frame_time: Instant,
    last_draw_calls: u32,
    last_triangles: u32,
    log_buffer: editor::console::LogBuffer,
    play_time: f32,
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
            last_frame_time: Instant::now(),
            last_draw_calls: 0,
            last_triangles: 0,
            log_buffer: log_buffer.clone(),
            play_time: 0.0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("ClawdEngine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_min_inner_size(winit::dpi::LogicalSize::new(1024, 600));
        let window =
            Arc::new(event_loop.create_window(attrs).expect("Failed to create window"));

        // macOS workaround (wgpu #5722): request_redraw BEFORE create_surface
        window.request_redraw();

        let mut gpu = GpuContext::new(window.clone());

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            None,
        );

        // Create scene renderer with 3D pipeline
        let mut scene = SceneRenderer::new(&mut gpu);

        // Load built-in procedural meshes
        let (cube_verts, cube_idx) = renderer::mesh::generate_cube();
        let cube_mesh_id = scene.mesh_store.add_named(&gpu.device, &cube_verts, &cube_idx, "builtin:cube");

        let (sphere_verts, sphere_idx) = renderer::mesh::generate_sphere(48, 64);
        let sphere_mesh_id = scene.mesh_store.add_named(&gpu.device, &sphere_verts, &sphere_idx, "builtin:sphere");

        log::info!("Loaded {} built-in meshes", scene.mesh_store.len());

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.scene = Some(scene);
        self.egui_state = Some(egui_state);
        self.editor_ctx = Some(editor::EditorContext::new(cube_mesh_id, sphere_mesh_id, self.log_buffer.clone()));

        editor::theme::apply_theme(&self.egui_ctx);

        // Load default showcase scene
        self.scripts = editor::default_scene::setup_default_scene(
            &mut self.world,
            cube_mesh_id,
            sphere_mesh_id,
        );

        // Populate script registry
        if let Some(ec) = &mut self.editor_ctx {
            ec.script_registry = vec![
                ScriptRegistryEntry {
                    name: "Rotate & Pulse".into(),
                    factory: Box::new(|| Box::new(scripting::RotateAndPulse::new())),
                },
                ScriptRegistryEntry {
                    name: "Gravity Bounce".into(),
                    factory: Box::new(|| Box::new(scripting::GravityBounce::new())),
                },
                ScriptRegistryEntry {
                    name: "Color Cycle".into(),
                    factory: Box::new(|| Box::new(scripting::ColorCycle::new())),
                },
                ScriptRegistryEntry {
                    name: "Oscillate".into(),
                    factory: Box::new(|| Box::new(scripting::Oscillate::new())),
                },
                ScriptRegistryEntry {
                    name: "Audio Demo".into(),
                    factory: Box::new(|| Box::new(scripting::AudioDemo::new())),
                },
                ScriptRegistryEntry {
                    name: "Player HUD".into(),
                    factory: Box::new(|| Box::new(scripting::PlayerHUD::new())),
                },
                ScriptRegistryEntry {
                    name: "FPS Controller".into(),
                    factory: Box::new(|| Box::new(scripting::FPSController::new())),
                },
            ];

            // Scan existing .rs scripts in assets/ and register as UserScript
            fn scan_scripts(dir: &std::path::Path, registry: &mut Vec<ScriptRegistryEntry>) {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            scan_scripts(&path, registry);
                        } else if path.extension().is_some_and(|e| e == "rs") {
                            if let Some(stem) = path.file_stem() {
                                let name = stem.to_string_lossy().into_owned();
                                let n = name.clone();
                                registry.push(ScriptRegistryEntry {
                                    name,
                                    factory: Box::new(move || {
                                        Box::new(scripting::UserScript::new(n.clone()))
                                    }),
                                });
                            }
                        }
                    }
                }
            }
            scan_scripts(std::path::Path::new("assets"), &mut ec.script_registry);
        }

        // Init audio system
        self.audio = audio::AudioSystem::new();
        if self.audio.is_some() {
            log::info!("Audio system initialized");
        } else {
            log::warn!("No audio device found, audio disabled");
        }

        // Scan asset files for Asset Browser
        if let Some(ec) = &mut self.editor_ctx {
            ec.refresh_assets();
        }

        log::info!("Window, GPU, and egui initialized");
        log::info!(
            "Spawned {} entities, {} with transforms",
            self.world.entity_count(),
            self.world.transforms_iter().count()
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Always route mouse/scroll to input (egui viewport consumes these)
        match &event {
            WindowEvent::CursorMoved { position, .. } => {
                self.input.on_cursor_moved(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.on_mouse_button(*button, *state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.input.on_scroll(*x, *y);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        self.input.on_scroll(p.x as f32 / 120.0, p.y as f32 / 120.0);
                    }
                }
            }
            _ => {}
        }

        // Always route keyboard to Input (modifiers must be tracked even when egui consumes)
        if let WindowEvent::KeyboardInput { event: key_event, .. } = &event {
            if let winit::keyboard::PhysicalKey::Code(code) = key_event.physical_key {
                self.input.on_keyboard(code, key_event.state);
            }
            // Track logical key (layout-aware) for Cmd+Z/S/O/D shortcuts on AZERTY etc.
            if let winit::keyboard::Key::Character(ref s) = key_event.logical_key {
                if let Some(c) = s.chars().next() {
                    self.input.on_logical_key(c.to_ascii_lowercase(), key_event.state);
                }
            }
        }

        // Route events to egui
        if let (Some(window), Some(egui_state)) = (&self.window, &mut self.egui_state) {
            let response = egui_state.on_window_event(window, &event);
            if response.consumed {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested, exiting");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
            }
            _ => {}
        }
    }
}

impl App {
    fn handle_redraw(&mut self) {
        // FPS calculation
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        if let Some(ec) = &mut self.editor_ctx {
            let new_fps = 1.0 / dt.max(0.001);
            ec.fps = ec.fps * 0.95 + new_fps * 0.05;
            ec.entity_count = self.world.entity_count();
            ec.draw_calls = self.last_draw_calls;
            ec.visible_triangles = self.last_triangles;
        }

        // Editor shortcuts
        let wants_kb = self.egui_ctx.wants_keyboard_input();
        let right_held = self.input.is_mouse_held(MouseButton::Right);
        let focus_requested = if let Some(ec) = &mut self.editor_ctx {
            editor::shortcuts::handle_shortcuts(&self.input, ec, wants_kb, right_held)
        } else {
            false
        };

        // Camera controls
        self.update_camera(dt, focus_requested);

        // Gizmo interaction
        self.update_gizmo();

        // Physics step (Play mode only)
        let is_playing = self.editor_ctx.as_ref().is_some_and(|ec| ec.play_mode);
        if is_playing {
            self.collision_events = physics::PhysicsSystem::step(
                &mut self.world,
                &mut self.collision_state,
                dt.max(0.001),
            );
        }

        // Audio update (Play mode)
        if let Some(ref mut audio_sys) = self.audio {
            audio_sys.update(&mut self.world, is_playing);
        }

        // Script execution (Play mode only)
        if is_playing {
            let dt_clamped = dt.max(0.001);
            let mut scripts = std::mem::take(&mut self.scripts);
            let mut pending_destroy = Vec::new();

            if !self.scripts_started {
                self.play_time = 0.0;
                for (eid, script) in &mut scripts {
                    let mut ctx = scripting::ScriptContext::new_full(
                        *eid, &mut self.world, &[],
                        &self.input, 0.0, dt_clamped,
                    );
                    script.start(&mut ctx);
                }
                self.scripts_started = true;
            }

            self.play_time += dt_clamped;
            for (eid, script) in &mut scripts {
                let mut ctx = scripting::ScriptContext::new_full(
                    *eid, &mut self.world, &self.collision_events,
                    &self.input, self.play_time, dt_clamped,
                );
                script.update(&mut ctx, dt_clamped);
                pending_destroy.extend(ctx.take_pending_destroy());
            }

            self.scripts = scripts;

            // Apply deferred entity destruction
            for eid in pending_destroy {
                self.world.destroy_entity(eid);
                self.scripts.retain(|(id, _)| *id != eid);
            }
        }

        // Resize viewport texture if dock panel changed size
        if let (Some(gpu), Some(scene), Some(ec)) =
            (&mut self.gpu, &mut self.scene, &self.editor_ctx)
        {
            let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32);

            // Scene viewport resize
            let vp = ec.viewport_rect;
            let new_w = (vp.width() * scale).max(1.0) as u32;
            let new_h = (vp.height() * scale).max(1.0) as u32;
            if new_w != scene.viewport.width || new_h != scene.viewport.height {
                scene.viewport.resize(
                    &gpu.device,
                    &mut gpu.egui_renderer,
                    new_w,
                    new_h,
                );
            }

            // Game viewport resize
            let gvp = ec.game_viewport_rect;
            let gw = (gvp.width() * scale).max(1.0) as u32;
            let gh = (gvp.height() * scale).max(1.0) as u32;
            if let Some(game_vp) = &mut scene.game_viewport {
                if gw != game_vp.width || gh != game_vp.height {
                    game_vp.resize(&gpu.device, &mut gpu.egui_renderer, gw, gh);
                }
            }
        }

        // Render frame
        let (frame_dc, frame_tri) = self.render_frame();
        self.last_draw_calls = frame_dc;
        self.last_triangles = frame_tri;

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

        // Clear per-frame input AFTER processing
        self.input.begin_frame();

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn update_camera(&mut self, dt: f32, focus_requested: bool) {
        let Some(scene) = &mut self.scene else { return };

        // Only allow camera controls when mouse is inside the viewport
        let mouse_in_viewport = if let (Some(ec), Some(window)) = (&self.editor_ctx, &self.window) {
            let scale = window.scale_factor() as f32;
            let mp = self.input.mouse_position();
            let logical = egui::pos2(mp[0] / scale, mp[1] / scale);
            ec.viewport_rect.contains(logical)
        } else {
            true
        };

        // Right-click + drag -> orbit + WASD fly mode
        if self.input.is_mouse_held(MouseButton::Right) && mouse_in_viewport {
            let delta = self.input.mouse_delta();
            scene.camera.rotate(delta[0], delta[1]);

            use winit::keyboard::KeyCode;
            let speed = 5.0 * dt;
            let mut fwd = 0.0_f32;
            let mut right = 0.0_f32;
            let mut up = 0.0_f32;
            if self.input.is_key_held(KeyCode::KeyW) { fwd += speed; }
            if self.input.is_key_held(KeyCode::KeyS) { fwd -= speed; }
            if self.input.is_key_held(KeyCode::KeyD) { right += speed; }
            if self.input.is_key_held(KeyCode::KeyA) { right -= speed; }
            if self.input.is_key_held(KeyCode::KeyE) { up += speed; }
            if self.input.is_key_held(KeyCode::KeyQ) { up -= speed; }
            if fwd.abs() > 0.0 || right.abs() > 0.0 || up.abs() > 0.0 {
                scene.camera.fly(fwd, right, up);
            }
        }

        // Middle-click + drag -> pan
        if self.input.is_mouse_held(MouseButton::Middle) && mouse_in_viewport {
            let delta = self.input.mouse_delta();
            scene.camera.pan(delta[0], delta[1]);
        }

        // Scroll -> zoom (only in viewport)
        if mouse_in_viewport {
            let scroll = self.input.scroll_delta();
            if scroll[1].abs() > 0.001 {
                scene.camera.zoom(scroll[1]);
            }
        }

        // F key -> focus on selected entity
        if focus_requested {
            if let Some(editor_ctx) = &self.editor_ctx {
                if let Some(eid) = editor_ctx.primary_selection() {
                    if let Some(transform) = self.world.get_transform(eid) {
                        scene.camera.focus_on(transform.position);
                    }
                }
            }
        }
    }

    fn update_gizmo(&mut self) {
        let (Some(window), Some(scene), Some(editor_ctx)) =
            (&self.window, &self.scene, &mut self.editor_ctx)
        else {
            return;
        };

        let scale = window.scale_factor() as f32;
        let mouse_pos = self.input.mouse_position();
        let logical_pos = egui::pos2(mouse_pos[0] / scale, mouse_pos[1] / scale);

        if editor_ctx.viewport_rect.contains(logical_pos) {
            let vp = editor_ctx.viewport_rect;
            let mx = logical_pos.x - vp.min.x;
            let my = logical_pos.y - vp.min.y;

            // Left-click: try gizmo pick first, then entity pick
            if self.input.is_mouse_pressed(MouseButton::Left) {
                let shift_held = self.input.is_key_held(winit::keyboard::KeyCode::ShiftLeft)
                    || self.input.is_key_held(winit::keyboard::KeyCode::ShiftRight);
                editor::gizmo_interaction::handle_gizmo_press(
                    editor_ctx, &self.world, scene, mx, my,
                    vp.width(), vp.height(), shift_held,
                );
            }

            // Left-held: drag gizmo
            if self.input.is_mouse_held(MouseButton::Left) {
                editor::gizmo_interaction::handle_gizmo_drag(
                    editor_ctx, &mut self.world, scene, mx, my,
                    vp.width(), vp.height(),
                );
            }

            // Hover detection
            editor::gizmo_interaction::update_hovered_axis(
                editor_ctx, &self.world, scene, mx, my,
                vp.width(), vp.height(),
            );
        }

        // Left-release: end gizmo drag
        if !self.input.is_mouse_held(MouseButton::Left) {
            editor::gizmo_interaction::handle_gizmo_release(editor_ctx);
        }
    }

    fn render_frame(&mut self) -> (u32, u32) {
        let selected = self.editor_ctx.as_ref().and_then(|ec| ec.primary_selection());
        let gizmo_pos = self.editor_ctx.as_ref().and_then(|ec| ec.selection_center(&self.world));

        let (Some(gpu), Some(window), Some(egui_state), Some(scene)) = (
            &mut self.gpu,
            &self.window,
            &mut self.egui_state,
            &mut self.scene,
        ) else {
            return (0, 0);
        };

        // Ensure game viewport texture exists if the tab was visible last frame
        let game_view_was_visible = self.editor_ctx.as_ref()
            .map_or(false, |ec| ec.game_view_visible);
        let game_vp_tex = if game_view_was_visible {
            Some(scene.ensure_game_viewport(&gpu.device, &mut gpu.egui_renderer))
        } else {
            scene.game_viewport.as_ref().map(|gv| gv.egui_texture_id)
        };

        // Reset game_view_visible for this frame (egui will set it again if tab is shown)
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
        let show_grid = editor_ctx.as_ref().map_or(true, |ec| ec.show_grid);
        let render_game = game_view_was_visible;

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
            |ctx, world| {
                if let Some(ec) = editor_ctx {
                    editor::EditorLayout::show(
                        ctx,
                        Some(viewport_tex_id),
                        game_vp_tex,
                        world,
                        ec,
                        scripts,
                    );
                }
            },
        )
    }
}

fn main() {
    let log_buffer = editor::console::init_logger();
    log::info!("ClawdEngine starting...");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new(log_buffer);
    event_loop.run_app(&mut app).expect("Event loop error");
}
