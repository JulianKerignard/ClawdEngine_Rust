use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::App;

impl App {
    pub(crate) fn handle_window_event(
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

        // Player mode: ESC quits the game
        if self.player_scene.is_some() {
            if let WindowEvent::KeyboardInput { event: ref key_event, .. } = event {
                if key_event.state == winit::event::ElementState::Pressed
                    && key_event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
                {
                    log::info!("ESC pressed in player mode — exiting");
                    event_loop.exit();
                    return;
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested, exiting");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                log::debug!("[Window] Resized to {}x{}", size.width, size.height);
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::DroppedFile(path) => {
                let path_str = path.to_string_lossy().to_string();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                log::info!("[Drop] File dropped: '{}' (ext=.{}, size={} bytes)", path_str, ext, file_size);
                match ext {
                    "fbx" | "glb" | "gltf" | "obj" => {
                        log::info!("[Drop] 3D model detected (.{}), initiating import pipeline", ext);
                        if let Some(ref mut ec) = self.editor_ctx {
                            // Copy file to project folder, use local path for loading
                            let load_path = crate::editor::operations::asset_ops::copy_dropped_file_to_project(ec, &path_str)
                                .unwrap_or(path_str.clone());
                            log::debug!("[Drop] Load path resolved: '{}'", load_path);

                            // Navigate asset browser to project root and highlight the imported file
                            ec.asset_current_dir = std::path::PathBuf::from(".");
                            if let Some(fname) = std::path::Path::new(&load_path).file_name() {
                                ec.asset_highlight_file = Some(fname.to_string_lossy().into_owned());
                                ec.asset_highlight_timer = 3.0;
                            }
                            ec.refresh_assets();

                            ec.pending_load_asset = Some(load_path);
                        } else {
                            log::warn!("[Drop] No editor context — cannot load asset");
                        }
                    }
                    "png" | "jpg" | "jpeg" | "tga" | "bmp" | "hdr" | "wav" | "mp3" | "ogg" | "flac" => {
                        log::info!("[Drop] Asset file (.{}), copying to project", ext);
                        if let Some(ref mut ec) = self.editor_ctx {
                            if let Some(local_path) = crate::editor::operations::asset_ops::copy_dropped_file_to_project(ec, &path_str) {
                                ec.asset_current_dir = std::path::PathBuf::from(".");
                                if let Some(fname) = std::path::Path::new(&local_path).file_name() {
                                    ec.asset_highlight_file = Some(fname.to_string_lossy().into_owned());
                                    ec.asset_highlight_timer = 3.0;
                                }
                                ec.refresh_assets();
                            }
                        }
                    }
                    _ => {
                        log::warn!("[Drop] Unsupported file type: .{} — ignoring", ext);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
            }
            _ => {}
        }
    }
}
