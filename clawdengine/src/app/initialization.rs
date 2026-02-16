use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::assets;
use crate::audio;
use crate::core;
use crate::editor;
use crate::editor::context::ScriptRegistryEntry;
use crate::renderer::{GpuContext, SceneRenderer};
use crate::scripting;

use crate::App;

impl App {
    pub(crate) fn init_engine(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_title = self.player_game_name.as_deref().unwrap_or("ClawdEngine");
        let attrs = Window::default_attributes()
            .with_title(window_title)
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_min_inner_size(winit::dpi::LogicalSize::new(1024, 600));
        let window =
            Arc::new(event_loop.create_window(attrs).expect("Failed to create window"));

        // macOS workaround (wgpu #5722): request_redraw BEFORE create_surface
        window.request_redraw();

        // Initialize asset path resolution (detects .app bundle vs dev mode)
        assets::paths::init();

        // In .app bundle: set CWD to Resources so relative asset paths work
        if assets::paths::is_bundled() {
            let _ = std::env::set_current_dir(assets::paths::base_dir());
        }

        let mut gpu = GpuContext::new(window.clone());

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            None,
        );

        let mut scene = SceneRenderer::new(&mut gpu);

        // Load built-in procedural meshes
        let mesh_ids = Self::load_builtin_meshes(&mut scene, &gpu);

        log::info!("Loaded {} built-in meshes", scene.mesh_store.len());

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.scene = Some(scene);
        self.egui_state = Some(egui_state);
        self.editor_ctx = Some(editor::EditorContext::new(
            mesh_ids.0, mesh_ids.1, mesh_ids.2,
            mesh_ids.3, mesh_ids.4, mesh_ids.5,
            self.log_buffer.clone(),
        ));

        editor::theme::apply_theme(&self.egui_ctx);

        self.register_scripts();
        self.init_audio();
        self.detect_player_mode();
        self.load_player_scene_or_assets();

        if self.player_scene.is_some() {
            log::info!("Game player started");
        } else {
            log::info!("Window, GPU, and egui initialized — showing Project Hub");
        }
    }

    fn load_builtin_meshes(
        scene: &mut SceneRenderer,
        gpu: &GpuContext,
    ) -> (usize, usize, usize, usize, usize, usize) {
        use crate::renderer::mesh;

        let (cube_verts, cube_idx) = mesh::generate_cube();
        let cube = scene.mesh_store.add_named(&gpu.device, &cube_verts, &cube_idx, "builtin:cube");

        let (sphere_verts, sphere_idx) = mesh::generate_sphere(48, 64);
        let sphere = scene.mesh_store.add_named(&gpu.device, &sphere_verts, &sphere_idx, "builtin:sphere");

        let (plane_verts, plane_idx) = mesh::generate_plane();
        let plane = scene.mesh_store.add_named(&gpu.device, &plane_verts, &plane_idx, "builtin:plane");

        let (cyl_verts, cyl_idx) = mesh::generate_cylinder(32);
        let cylinder = scene.mesh_store.add_named(&gpu.device, &cyl_verts, &cyl_idx, "builtin:cylinder");

        let (cap_verts, cap_idx) = mesh::generate_capsule(32, 16);
        let capsule = scene.mesh_store.add_named(&gpu.device, &cap_verts, &cap_idx, "builtin:capsule");

        let (cone_verts, cone_idx) = mesh::generate_cone(32);
        let cone = scene.mesh_store.add_named(&gpu.device, &cone_verts, &cone_idx, "builtin:cone");

        (cube, sphere, plane, cylinder, capsule, cone)
    }

    fn register_scripts(&mut self) {
        let Some(ec) = &mut self.editor_ctx else { return };

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

    fn init_audio(&mut self) {
        self.audio = audio::AudioSystem::new();
        if self.audio.is_some() {
            log::info!("Audio system initialized");
        } else {
            log::warn!("No audio device found, audio disabled");
        }
    }

    fn detect_player_mode(&mut self) {
        if self.player_scene.is_none() {
            if let Ok(game_manifest) = assets::project::load_game_manifest() {
                log::info!("Detected game.ron — entering player mode: {}", game_manifest.name);
                self.player_scene = Some(game_manifest.startup_scene);
                self.player_game_name = Some(game_manifest.name.clone());
                if let Some(window) = &self.window {
                    window.set_title(&game_manifest.name);
                }
            }
        }
    }

    fn load_player_scene_or_assets(&mut self) {
        if let Some(scene_path) = &self.player_scene {
            let scene_path = scene_path.clone();
            let resolved = assets::paths::resolve(&scene_path);
            let path_str = resolved.to_string_lossy().to_string();
            log::info!("Player mode — loading scene: {}", path_str);

            if let (Some(ec), Some(scene), Some(gpu)) =
                (&mut self.editor_ctx, &mut self.scene, &self.gpu)
            {
                ec.screen = editor::context::AppScreen::Editor;
                ec.play_mode = true;
                ec.fullscreen_game = true;

                if let Err(e) = assets::scene::load_scene(
                    &mut self.world,
                    &mut scene.mesh_store,
                    &mut self.scripts,
                    &ec.script_registry,
                    &gpu.device,
                    &path_str,
                ) {
                    log::error!("Failed to load scene '{}': {}", path_str, e);
                } else {
                    self.resolve_material_textures();
                    log::info!("Scene loaded: {} entities", self.world.entity_count());
                    self.ensure_main_camera();
                }
            }
        } else {
            // Scan asset files for Asset Browser (editor mode only)
            if let Some(ec) = &mut self.editor_ctx {
                ec.refresh_assets();
            }
        }
    }

    fn resolve_material_textures(&mut self) {
        let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu) else { return };
        let entity_ids: Vec<_> = self.world.iter_entities().collect();
        for eid in entity_ids {
            if let Some(mat) = self.world.get_material_mut(eid) {
                if let Some(path) = mat.texture_path.clone() {
                    let tid = scene.texture_store.load(&gpu.device, &gpu.queue, &path);
                    mat.texture_id = Some(tid);
                }
                if let Some(path) = mat.normal_map_path.clone() {
                    let nid = scene.texture_store.load(&gpu.device, &gpu.queue, &path);
                    mat.normal_map_id = Some(nid);
                }
            }
        }
    }

    fn ensure_main_camera(&mut self) {
        let Some(scene) = &self.scene else { return };
        let has_main_cam = self.world.iter_entities()
            .any(|eid| self.world.get_camera(eid).is_some_and(|c| c.is_main));
        if !has_main_cam {
            let cam_id = self.world.spawn_entity();
            self.world.set_name(cam_id, "Main Camera");
            self.world.set_transform(cam_id, core::Transform {
                position: scene.camera.eye(),
                rotation: glam::Quat::from_rotation_arc(
                    -glam::Vec3::Z,
                    scene.camera.forward(),
                ),
                ..Default::default()
            });
            self.world.set_camera(cam_id, core::CameraComponent::default());
            log::info!("Auto-created main camera at {:?}", scene.camera.eye());
        }
    }
}
