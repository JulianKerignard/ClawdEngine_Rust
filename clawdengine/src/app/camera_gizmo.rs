use winit::event::MouseButton;

use crate::editor;

use crate::App;

impl App {
    pub(crate) fn update_camera(&mut self, dt: f32, focus_requested: bool) {
        let Some(scene) = &mut self.scene else { return };

        // Apply camera settings from project settings
        if let Some(ec) = &self.editor_ctx {
            let cam = &ec.project_settings.camera;
            scene.camera.fov_y = cam.fov.to_radians();
            scene.camera.near = cam.near_clip;
            scene.camera.far = cam.far_clip;
        }

        // Only allow camera controls when mouse is inside the viewport
        // Use is_using_pointer() (active drag on egui widget) instead of wants_pointer_input()
        // which blocks ALL interaction when pointer is over the dock area
        let mouse_in_viewport = if self.egui_ctx.is_using_pointer() {
            false
        } else if let (Some(ec), Some(window)) = (&self.editor_ctx, &self.window) {
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
            let cam_sens = self.editor_ctx.as_ref()
                .map(|ec| ec.project_settings.camera.rotate_sensitivity)
                .unwrap_or(0.005);
            scene.camera.rotate(delta[0], delta[1], cam_sens);

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
            let pan_sens = self.editor_ctx.as_ref()
                .map(|ec| ec.project_settings.camera.pan_sensitivity)
                .unwrap_or(0.005);
            scene.camera.pan(delta[0], delta[1], pan_sens);
        }

        // Scroll -> zoom (only in viewport)
        if mouse_in_viewport {
            let scroll = self.input.scroll_delta();
            if scroll[1].abs() > 0.001 {
                let zoom_spd = self.editor_ctx.as_ref()
                    .map(|ec| ec.project_settings.camera.zoom_speed)
                    .unwrap_or(0.5);
                scene.camera.zoom(scroll[1], zoom_spd);
            }
        }

        // F key -> focus on selected entity
        if focus_requested {
            if let Some(editor_ctx) = &self.editor_ctx {
                if let Some(eid) = editor_ctx.primary_selection() {
                    let pos = self.world.get_world_transform(eid)
                        .or_else(|| self.world.get_transform(eid).copied())
                        .map(|t| t.position);
                    if let Some(p) = pos {
                        scene.camera.focus_on(p);
                    }
                }
            }
        }

        // Auto-focus after model import
        if let Some(ec) = &mut self.editor_ctx {
            if let Some(target) = ec.pending_camera_focus.take() {
                scene.camera.focus_on(target);
            }
        }
    }

    pub(crate) fn update_gizmo(&mut self) {
        let (Some(window), Some(scene), Some(editor_ctx)) =
            (&self.window, &self.scene, &mut self.editor_ctx)
        else {
            return;
        };

        // Don't interact with gizmo when egui is actively dragging a widget
        if self.egui_ctx.is_using_pointer() {
            editor::gizmo_interaction::handle_gizmo_release(editor_ctx);
            return;
        }

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
}
