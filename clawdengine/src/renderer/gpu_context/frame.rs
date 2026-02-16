use winit::window::Window;

use crate::core::{EntityId, World};
use crate::renderer::scene_helpers;

use super::{GpuContext, SceneRenderer};

impl GpuContext {
    #[allow(clippy::too_many_arguments)]
    pub fn render_frame(
        &mut self,
        egui_ctx: &egui::Context,
        egui_state: &mut egui_winit::State,
        window: &Window,
        scene: &mut SceneRenderer,
        world: &mut World,
        selected_entity: Option<EntityId>,
        gizmo_pos: Option<glam::Vec3>,
        active_tool: crate::editor::context::EditorTool,
        gizmo_drag_axis: Option<crate::editor::context::GizmoAxis>,
        gizmo_hover_axis: Option<crate::editor::context::GizmoAxis>,
        show_grid: bool,
        render_game: bool,
        settings: &crate::assets::settings::ProjectSettings,
        mut ui_fn: impl FnMut(&egui::Context, &mut World, &SceneRenderer),
    ) -> (u32, u32) {
        let output = match self.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return (0, 0);
            }
            Err(e) => {
                log::error!("Surface error: {e}");
                return (0, 0);
            }
        };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        // ---- Pass 1: 3D scene -> viewport texture ----
        let (dc, tri) = self.render_3d_pass(
            &mut encoder, scene, world, selected_entity, gizmo_pos,
            active_tool, gizmo_drag_axis, gizmo_hover_axis, show_grid, settings,
        );

        // ---- Pass 1b: Game view -> game viewport texture ----
        if render_game {
            self.render_game_view(&mut encoder, scene, world);
        }

        // ---- Egui frame ----
        let raw_input = egui_state.take_egui_input(window);
        let scene_ref: &SceneRenderer = &*scene;
        let full_output = egui_ctx.run(raw_input, |ctx| ui_fn(ctx, world, scene_ref));
        egui_state.handle_platform_output(window, full_output.platform_output);

        let paint_jobs =
            egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        let user_cmd_bufs = self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // ---- Pass 2: Clear surface + egui overlay ----
        {
            let egui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.118,
                            g: 0.118,
                            b: 0.118,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(
                &mut egui_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        // Submit
        let encoded = encoder.finish();
        self.queue
            .submit(user_cmd_bufs.into_iter().chain(std::iter::once(encoded)));
        output.present();

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        (dc, tri)
    }

    pub fn render_3d_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &mut SceneRenderer,
        world: &World,
        selected_entity: Option<EntityId>,
        gizmo_position: Option<glam::Vec3>,
        active_tool: crate::editor::context::EditorTool,
        gizmo_drag_axis: Option<crate::editor::context::GizmoAxis>,
        gizmo_hover_axis: Option<crate::editor::context::GizmoAxis>,
        show_grid: bool,
        settings: &crate::assets::settings::ProjectSettings,
    ) -> (u32, u32) {
        let cam_uniforms = scene.camera.uniforms(scene.viewport.aspect_ratio());
        self.queue.write_buffer(
            &scene.camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_uniforms]),
        );

        scene_helpers::build_light_uniforms(world, &self.queue, scene, settings.lighting.ambient_color);
        scene_helpers::update_shadow_vp(
            world, &self.queue, scene,
            settings.shadows.light_distance,
            settings.shadows.ortho_size,
            settings.shadows.near_plane,
            settings.shadows.far_plane,
        );

        scene.line_batch.clear();
        let cam_eye = scene.camera.eye();
        let eye = [cam_eye.x, cam_eye.y, cam_eye.z];

        scene_helpers::draw_gizmos(
            &mut scene.line_batch, gizmo_position, active_tool,
            gizmo_drag_axis, gizmo_hover_axis, eye,
        );

        let cam_forward = -scene.camera.forward();
        let cam_forward_n = cam_forward.normalize_or_zero();
        let cam_right = cam_forward_n.cross(glam::Vec3::Y).normalize_or_zero();
        let cam_up = cam_right.cross(cam_forward_n).normalize_or_zero();

        scene_helpers::draw_light_helpers(
            &mut scene.line_batch, world, selected_entity,
            cam_right, cam_up, eye,
        );
        scene_helpers::draw_camera_helpers(
            &mut scene.line_batch, world, selected_entity,
            cam_right, cam_up, eye,
        );
        scene_helpers::draw_grid(&mut scene.line_batch, show_grid, settings.grid.half_size, settings.grid.color);

        if let Some(sel_eid) = selected_entity {
            scene_helpers::draw_collider_debug(
                &mut scene.line_batch, world, &[sel_eid],
            );
        }

        scene.line_batch.upload(&self.device, &self.queue);

        let (renderables, per_entity_data) =
            self.prepare_entity_data(scene, world, selected_entity);

        self.execute_shadow_pass(encoder, scene, &renderables, &per_entity_data);
        self.execute_main_pass(
            encoder, scene,
            &scene.camera_bind_group,
            &scene.viewport.msaa_color_view,
            &scene.viewport.color_view,
            &scene.viewport.depth_view,
            &renderables, &per_entity_data,
            true,
        )
    }

    pub fn render_game_view(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneRenderer,
        world: &World,
    ) -> (u32, u32) {
        let game_vp = match &scene.game_viewport {
            Some(vp) => vp,
            None => return (0, 0),
        };

        let cam_uniforms = match Self::find_main_camera(world, game_vp.aspect_ratio()) {
            Some(u) => u,
            None => return (0, 0),
        };

        self.queue.write_buffer(
            &scene.game_camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_uniforms]),
        );

        let (renderables, per_entity_data) = self.prepare_entity_data(scene, world, None);
        self.execute_shadow_pass(encoder, scene, &renderables, &per_entity_data);
        self.execute_main_pass(
            encoder,
            scene,
            &scene.game_camera_bind_group,
            &game_vp.msaa_color_view,
            &game_vp.color_view,
            &game_vp.depth_view,
            &renderables,
            &per_entity_data,
            false,
        )
    }

    fn find_main_camera(world: &World, aspect: f32) -> Option<crate::renderer::camera::CameraUniforms> {
        for eid in world.iter_entities() {
            let Some(cam) = world.get_camera(eid) else { continue };
            if !cam.is_main { continue; }
            let wt = world.get_world_transform(eid)?;

            let fwd = wt.rotation * glam::Vec3::new(0.0, 0.0, -1.0);
            let eye = wt.position;
            let target = eye + fwd;
            let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
            let proj = glam::Mat4::perspective_rh(cam.fov_y, aspect, cam.near, cam.far);

            return Some(crate::renderer::camera::CameraUniforms {
                view_proj: (proj * view).to_cols_array_2d(),
                eye_position: [eye.x, eye.y, eye.z, 1.0],
            });
        }
        None
    }
}
