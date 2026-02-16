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
                if key_event.state == winit::event::ElementState::Pressed {
                    if key_event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
                        log::info!("ESC pressed in player mode — exiting");
                        event_loop.exit();
                        return;
                    }
                }
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
