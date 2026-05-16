use std::sync::Arc;
use winit::window::Window;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::line_pipeline::{LineBatch, LinePipeline};
use super::mesh::MeshStore;
use super::pipeline::{LightData, LightsUniforms, MaterialUniforms, MeshPipeline, ModelUniforms};
use super::scene_helpers::{self, SELECTION_TINT};
use super::shadow::ShadowMap;
use super::skybox::SkyboxPipeline;
use super::texture_store::TextureStore;
use super::viewport::ViewportTexture;
use crate::core::{EntityId, World};

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui_renderer: egui_wgpu::Renderer,
}

/// Per-entity GPU resources that persist across frames.
///
/// Previously the renderer recreated 2 buffers + 2 bind groups per entity per
/// frame — a wgpu anti-pattern that pressures the driver allocator. We now keep
/// them in `SceneRenderer.entity_cache` indexed by `EntityId.index`. Uniforms
/// are refreshed via `queue.write_buffer`; bind groups are only recreated when
/// the material's texture references actually change.
pub struct CachedEntityGpu {
    pub model_buffer: wgpu::Buffer,
    pub model_bg: wgpu::BindGroup,
    pub mat_buffer: wgpu::Buffer,
    pub mat_bg: wgpu::BindGroup,
    /// Generation of the entity this cache was built for. When a slot is
    /// recycled, the new entity will have a different generation and we
    /// rebuild from scratch.
    generation: u32,
    /// Material texture ids the bind group currently references. If the user
    /// changes the albedo or normal map, the bind group must be rebuilt.
    cached_albedo_id: Option<usize>,
    cached_normal_id: Option<usize>,
}

pub struct SceneRenderer {
    pub pipeline: MeshPipeline,
    pub line_pipeline: LinePipeline,
    pub line_batch: LineBatch,
    pub viewport: ViewportTexture,
    pub camera: Camera,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub lights_buffer: wgpu::Buffer,
    pub lights_bind_group: wgpu::BindGroup,
    pub mesh_store: MeshStore,
    pub texture_store: TextureStore,
    pub skybox: SkyboxPipeline,
    pub shadow_map: ShadowMap,
    pub game_viewport: Option<ViewportTexture>,
    pub game_camera_buffer: wgpu::Buffer,
    pub game_camera_bind_group: wgpu::BindGroup,
    /// Persistent GPU cache keyed by `EntityId.index`. Sparse — None for
    /// indices that never carried a renderable or that have been recycled but
    /// not yet rebuilt.
    pub entity_cache: Vec<Option<CachedEntityGpu>>,
}

fn create_camera_setup(
    device: &wgpu::Device,
    camera_bgl: &wgpu::BindGroupLayout,
    uniforms: &super::camera::CameraUniforms,
    label: &str,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label} Uniform Buffer")),
        contents: bytemuck::cast_slice(&[*uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{label} Bind Group")),
        layout: camera_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    (buffer, bind_group)
}

impl SceneRenderer {
    pub fn new(gpu: &mut GpuContext) -> Self {
        // Create shadow BGL first (needed by MeshPipeline layout)
        let shadow_bgl = ShadowMap::create_shadow_bgl(&gpu.device);
        let pipeline = MeshPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &shadow_bgl);
        let shadow_map = ShadowMap::new(&gpu.device, &pipeline.model_bgl, shadow_bgl);

        let line_pipeline = LinePipeline::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            &pipeline.camera_bgl,
        );
        let line_batch = LineBatch::new(&gpu.device);

        let viewport = ViewportTexture::new(&gpu.device, &mut gpu.egui_renderer, 800, 600);

        let camera = Camera::default();
        let uniforms = camera.uniforms(viewport.aspect_ratio());

        let (camera_buffer, camera_bind_group) = create_camera_setup(
            &gpu.device, &pipeline.camera_bgl, &uniforms, "Camera",
        );

        // Lights buffer (fixed size, updated each frame)
        let lights_uniforms = LightsUniforms {
            ambient: [0.12, 0.14, 0.18, 1.0],
            count: 0,
            _pad: [0.0; 3],
            lights: [LightData {
                position: [0.0; 4],
                color: [0.0; 4],
                direction: [0.0; 4],
                spot_params: [0.0; 4],
            }; 4],
        };

        let lights_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Uniform Buffer"),
            contents: bytemuck::cast_slice(&[lights_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lights_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lights Bind Group"),
            layout: &pipeline.lights_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buffer.as_entire_binding(),
            }],
        });

        let mesh_store = MeshStore::new();
        let texture_store = TextureStore::new(&gpu.device, &gpu.queue);

        let skybox = SkyboxPipeline::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            &pipeline.camera_bgl,
        );

        // Separate camera buffer for game view (avoids queue.write_buffer ordering issue)
        let (game_camera_buffer, game_camera_bind_group) = create_camera_setup(
            &gpu.device, &pipeline.camera_bgl, &uniforms, "Game Camera",
        );

        Self {
            pipeline,
            line_pipeline,
            line_batch,
            viewport,
            camera,
            camera_buffer,
            camera_bind_group,
            lights_buffer,
            lights_bind_group,
            mesh_store,
            texture_store,
            skybox,
            shadow_map,
            game_viewport: None,
            game_camera_buffer,
            game_camera_bind_group,
            entity_cache: Vec::new(),
        }
    }

    pub fn ensure_game_viewport(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> egui::TextureId {
        if self.game_viewport.is_none() {
            self.game_viewport = Some(ViewportTexture::new(device, egui_renderer, 800, 600));
        }
        self.game_viewport.as_ref().unwrap().egui_texture_id
    }
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create GPU surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("No compatible GPU adapter found");

        log::info!("GPU adapter: {}", adapter.get_info().name);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ClawdEngine Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 5,
                ..wgpu::Limits::default()
            },
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .expect("Failed to create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("Surface configuration unsupported by adapter");
        config.present_mode = wgpu::PresentMode::AutoVsync;
        config.format = match config.format {
            wgpu::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8Unorm,
            other => other,
        };
        config.view_formats.push(config.format);
        surface.configure(&device, &config);

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: true,
                predictable_texture_filtering: false,
            },
        );

        Self {
            surface,
            device,
            queue,
            config,
            egui_renderer,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

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
        mut ui_fn: impl FnMut(&egui::Context, &mut World),
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

        // ---- Pass 1: 3D scene → viewport texture ----
        let (dc, tri) = self.render_3d_pass(&mut encoder, scene, world, selected_entity, gizmo_pos, active_tool, gizmo_drag_axis, gizmo_hover_axis, show_grid);

        // ---- Pass 1b: Game view → game viewport texture ----
        if render_game {
            self.render_game_view(&mut encoder, scene, world);
        }

        // ---- Egui frame ----
        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| ui_fn(ctx, world));
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

        // Free egui textures
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
    ) -> (u32, u32) {
        // Update camera uniforms
        let cam_uniforms = scene.camera.uniforms(scene.viewport.aspect_ratio());
        self.queue.write_buffer(
            &scene.camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_uniforms]),
        );

        // Upload lights and shadow VP
        scene_helpers::build_light_uniforms(world, &self.queue, scene);
        scene_helpers::update_shadow_vp(world, &self.queue, scene);

        // Build line batch (gizmos, light helpers, grid)
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

        scene_helpers::draw_grid(&mut scene.line_batch, show_grid);

        // Collider wireframe for selected entities
        if let Some(sel_eid) = selected_entity {
            scene_helpers::draw_collider_debug(
                &mut scene.line_batch, world, &[sel_eid],
            );
        }

        scene.line_batch.upload(&self.device, &self.queue);

        // Collect renderable entities and refresh the per-entity GPU cache.
        let renderables = self.prepare_entity_data(scene, world, selected_entity);

        // Execute shadow pass, then main 3D pass
        self.execute_shadow_pass(encoder, scene, &renderables);
        self.execute_main_pass(
            encoder, scene,
            &scene.camera_bind_group,
            &scene.viewport.msaa_color_view,
            &scene.viewport.color_view,
            &scene.viewport.depth_view,
            &renderables,
            true,
        )
    }

    /// Walk the world, build the list of renderable entities, and refresh /
    /// populate per-entity GPU resources in `scene.entity_cache`.
    ///
    /// On the steady-state hot path this only writes ~64 bytes per entity to
    /// existing buffers (one `queue.write_buffer` for model uniforms, one for
    /// material uniforms). Buffers and bind groups are created once and reused
    /// across frames; the material bind group is rebuilt only when the
    /// material's texture references change.
    fn prepare_entity_data<'a>(
        &self,
        scene: &'a mut SceneRenderer,
        world: &'a World,
        selected_entity: Option<EntityId>,
    ) -> Vec<(EntityId, usize)> {
        let renderables: Vec<(EntityId, usize, Option<usize>, Option<usize>)> = world
            .iter_entities()
            .filter_map(|eid| {
                let _ = world.get_transform(eid)?;
                let mesh_renderer = world.get_mesh_renderer(eid)?;
                if !mesh_renderer.visible {
                    return None;
                }
                let mesh_id = mesh_renderer.mesh_id?;
                let material = world.get_material(eid);
                let albedo_id = material.and_then(|m| m.texture_id);
                let normal_id = material.and_then(|m| m.normal_map_id);
                Some((eid, mesh_id, albedo_id, normal_id))
            })
            .collect();

        // Ensure the cache vector is large enough to index by eid.index.
        let max_index = renderables.iter().map(|(eid, _, _, _)| eid.index as usize).max();
        if let Some(mx) = max_index {
            if mx >= scene.entity_cache.len() {
                scene.entity_cache.resize_with(mx + 1, || None);
            }
        }

        // Eviction: drop cached GPU resources for slots that are no longer
        // renderable this frame (entity destroyed, mesh removed, made
        // invisible, or slot recycled). Without this the cache only ever
        // grows — destroyed entities would pin 2 buffers + 2 bind groups in
        // VRAM forever. The renderable set is small, so a HashSet keyed by
        // index is cheap; dropping the Option frees the wgpu resources.
        {
            let live: std::collections::HashSet<u32> =
                renderables.iter().map(|(eid, _, _, _)| eid.index).collect();
            for (i, slot) in scene.entity_cache.iter_mut().enumerate() {
                if slot.is_some() && !live.contains(&(i as u32)) {
                    *slot = None;
                }
            }
        }

        for (eid, _mesh_id, albedo_id, normal_id) in &renderables {
            let eid = *eid;
            let idx = eid.index as usize;

            // Compute the per-frame uniform values.
            let transform = match world.get_transform(eid) {
                Some(t) => *t,
                None => continue,
            };
            let wt = world.get_world_transform(eid).unwrap_or(transform);
            let model_matrix = glam::Mat4::from_scale_rotation_translation(
                wt.scale,
                wt.rotation,
                wt.position,
            );
            let color = if selected_entity == Some(eid) {
                SELECTION_TINT
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            let model_uniforms = ModelUniforms {
                model: model_matrix.to_cols_array_2d(),
                color,
            };

            let material = world.get_material(eid);
            let albedo = material
                .map(|m| [m.albedo.x, m.albedo.y, m.albedo.z, 1.0])
                .unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let emission = material
                .map(|m| [m.emission.x, m.emission.y, m.emission.z, 0.0])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let mat_uniforms = MaterialUniforms {
                albedo,
                roughness: material.map(|m| m.roughness).unwrap_or(0.5),
                metallic: material.map(|m| m.metallic).unwrap_or(0.0),
                _pad: [0.0; 2],
                emission,
            };

            // Decide whether the cached entry is reusable.
            let needs_rebuild = match &scene.entity_cache[idx] {
                None => true,
                Some(entry) => {
                    entry.generation != eid.generation
                        || entry.cached_albedo_id != *albedo_id
                        || entry.cached_normal_id != *normal_id
                }
            };

            if needs_rebuild {
                let model_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Model Uniform"),
                            contents: bytemuck::cast_slice(&[model_uniforms]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                let model_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Model BG"),
                    layout: &scene.pipeline.model_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: model_buffer.as_entire_binding(),
                    }],
                });

                let mat_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Material Uniform"),
                            contents: bytemuck::cast_slice(&[mat_uniforms]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                let gpu_tex = if let Some(tex_id) = *albedo_id {
                    scene.texture_store.get(tex_id)
                } else {
                    scene.texture_store.get(scene.texture_store.default_id())
                };

                let gpu_normal = if let Some(nid) = *normal_id {
                    scene.texture_store.get(nid)
                } else {
                    scene.texture_store.get(scene.texture_store.default_normal_id())
                };

                let mat_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Material BG"),
                    layout: &scene.pipeline.material_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: mat_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&gpu_tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&gpu_tex.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&gpu_normal.view),
                        },
                    ],
                });

                scene.entity_cache[idx] = Some(CachedEntityGpu {
                    model_buffer,
                    model_bg,
                    mat_buffer,
                    mat_bg,
                    generation: eid.generation,
                    cached_albedo_id: *albedo_id,
                    cached_normal_id: *normal_id,
                });
            } else {
                // Reuse existing buffers & bind groups; just refresh the data.
                let entry = scene.entity_cache[idx].as_ref().unwrap();
                self.queue
                    .write_buffer(&entry.model_buffer, 0, bytemuck::cast_slice(&[model_uniforms]));
                self.queue
                    .write_buffer(&entry.mat_buffer, 0, bytemuck::cast_slice(&[mat_uniforms]));
            }
        }

        renderables
            .into_iter()
            .map(|(eid, mesh_id, _, _)| (eid, mesh_id))
            .collect()
    }

    fn execute_shadow_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneRenderer,
        renderables: &[(EntityId, usize)],
    ) {
        let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shadow Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &scene.shadow_map.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        shadow_pass.set_pipeline(&scene.shadow_map.pipeline);
        shadow_pass.set_bind_group(0, &scene.shadow_map.light_vp_bind_group, &[]);

        for (eid, mesh_id) in renderables {
            let Some(entry) = scene.entity_cache.get(eid.index as usize).and_then(|e| e.as_ref()) else {
                continue;
            };
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                shadow_pass.set_bind_group(1, &entry.model_bg, &[]);
                shadow_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(
                    gpu_mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                shadow_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }
    }

    fn execute_main_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneRenderer,
        camera_bg: &wgpu::BindGroup,
        msaa_view: &wgpu::TextureView,
        resolve_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        renderables: &[(EntityId, usize)],
        draw_lines: bool,
    ) -> (u32, u32) {
        let mut draw_call_count: u32 = 0;
        let mut triangle_count: u32 = 0;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3D Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(resolve_view),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.165,
                        g: 0.165,
                        b: 0.165,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Skybox
        pass.set_pipeline(&scene.skybox.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);
        pass.draw(0..3, 0..1);

        // Mesh rendering
        pass.set_pipeline(&scene.pipeline.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);
        pass.set_bind_group(3, &scene.lights_bind_group, &[]);
        pass.set_bind_group(4, &scene.shadow_map.shadow_bind_group, &[]);

        for (eid, mesh_id) in renderables {
            let Some(entry) = scene.entity_cache.get(eid.index as usize).and_then(|e| e.as_ref()) else {
                continue;
            };
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                pass.set_bind_group(1, &entry.model_bg, &[]);
                pass.set_bind_group(2, &entry.mat_bg, &[]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(
                    gpu_mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
                draw_call_count += 1;
                triangle_count += gpu_mesh.index_count / 3;
            }
        }

        // Lines (editor overlays only)
        if draw_lines {
            if scene.line_batch.thin_count() > 0 {
                pass.set_pipeline(&scene.line_pipeline.thin);
                pass.set_bind_group(0, camera_bg, &[]);
                pass.set_vertex_buffer(0, scene.line_batch.thin_buffer().slice(..));
                pass.draw(0..scene.line_batch.thin_count(), 0..1);
            }
            if scene.line_batch.thick_count() > 0 {
                pass.set_pipeline(&scene.line_pipeline.thick);
                pass.set_bind_group(0, camera_bg, &[]);
                pass.set_vertex_buffer(0, scene.line_batch.thick_buffer().slice(..));
                pass.draw(0..scene.line_batch.thick_count(), 0..1);
            }
        }

        (draw_call_count, triangle_count)
    }

    pub fn render_game_view(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &mut SceneRenderer,
        world: &World,
    ) -> (u32, u32) {
        let aspect = match &scene.game_viewport {
            Some(vp) => vp.aspect_ratio(),
            None => return (0, 0),
        };

        let cam_uniforms = match Self::find_main_camera(world, aspect) {
            Some(u) => u,
            None => return (0, 0),
        };

        // Write to the SEPARATE game camera buffer (not the editor one)
        self.queue.write_buffer(
            &scene.game_camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_uniforms]),
        );

        // Refresh the per-entity GPU cache (no selection highlight in game view).
        let renderables = self.prepare_entity_data(scene, world, None);

        // From here on we only need an immutable borrow of `scene`.
        let scene: &SceneRenderer = scene;
        let game_vp = scene.game_viewport.as_ref().expect("game viewport disappeared mid-frame");

        self.execute_shadow_pass(encoder, scene, &renderables);
        self.execute_main_pass(
            encoder,
            scene,
            &scene.game_camera_bind_group,
            &game_vp.msaa_color_view,
            &game_vp.color_view,
            &game_vp.depth_view,
            &renderables,
            false,
        )
    }

    fn find_main_camera(world: &World, aspect: f32) -> Option<super::camera::CameraUniforms> {
        // World caches the main-camera EntityId across frames and invalidates
        // it on any camera mutation, so this is O(1) on the hot path.
        let eid = world.find_main_camera_entity()?;
        let cam = world.get_camera(eid)?;
        let wt = world.get_world_transform(eid)?;

        let fwd = wt.rotation * glam::Vec3::new(0.0, 0.0, -1.0);
        let eye = wt.position;
        let target = eye + fwd;
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(cam.fov_y, aspect, cam.near, cam.far);

        Some(super::camera::CameraUniforms {
            view_proj: (proj * view).to_cols_array_2d(),
            eye_position: [eye.x, eye.y, eye.z, 1.0],
        })
    }
}
