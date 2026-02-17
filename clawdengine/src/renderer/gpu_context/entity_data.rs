use wgpu::util::DeviceExt;

use crate::core::{EntityId, World};
use crate::renderer::frustum::Frustum;
use crate::renderer::pipeline::{MaterialUniforms, ModelUniforms};
use crate::renderer::scene_helpers::SELECTION_TINT;
use crate::renderer::texture_store::GpuTexture;

use super::{CachedEntityGpu, GpuContext, SceneRenderer};

fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    mat_buffer: &wgpu::Buffer,
    gpu_tex: &GpuTexture,
    gpu_normal: &GpuTexture,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Material BG"),
        layout,
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
    })
}

impl GpuContext {
    /// Prepare GPU data for visible entities. Returns `(renderables, culled_count)`.
    /// `culled_count` is the number of entities that had a mesh but were outside the frustum.
    pub(crate) fn prepare_entity_data<'w>(
        &self,
        scene: &mut SceneRenderer,
        world: &'w World,
        selected_entity: Option<EntityId>,
        frustum: &Frustum,
    ) -> (Vec<(EntityId, &'w crate::core::Transform, usize, Option<&'w crate::core::Material>)>, u32)
    {
        let mut culled = 0u32;
        let renderables: Vec<_> = world
            .iter_entities()
            .filter_map(|eid| {
                let transform = world.get_transform(eid)?;
                let mesh_renderer = world.get_mesh_renderer(eid)?;
                if !mesh_renderer.visible {
                    return None;
                }
                let mesh_id = mesh_renderer.mesh_id?;

                // Frustum culling: test world-space AABB against camera frustum
                if let Some(aabb) = scene.mesh_store.get_aabb(mesh_id) {
                    let wt = world.get_world_transform(eid).unwrap_or(*transform);
                    let model = glam::Mat4::from_scale_rotation_translation(
                        wt.scale, wt.rotation, wt.position,
                    );
                    let (wmin, wmax) = aabb.transformed(model);
                    if !frustum.intersects_aabb(wmin, wmax) {
                        culled += 1;
                        return None;
                    }
                }

                let material = world.get_material(eid);
                Some((eid, transform, mesh_id, material))
            })
            .collect();

        // Increment frame counter for cache staleness tracking
        scene.frame_counter = scene.frame_counter.wrapping_add(1);
        let current_frame = scene.frame_counter;

        // Take the cache out of scene so we can borrow scene fields immutably
        // while mutating the cache (standard Rust borrow-splitting pattern).
        let mut cache = std::mem::take(&mut scene.entity_gpu_cache);

        let buf_usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;

        for &(eid, transform, mesh_id, material) in &renderables {
            let wt = world.get_world_transform(eid).unwrap_or(*transform);
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

            let texture_id = material.and_then(|m| m.texture_id);
            let normal_map_id = material.and_then(|m| m.normal_map_id);

            // Pre-extract texture references (immutable borrows on scene fields)
            let gpu_tex = if let Some(tex_id) = texture_id {
                scene.texture_store.get(tex_id)
            } else {
                scene.texture_store.get(scene.texture_store.default_id())
            };
            let gpu_normal = if let Some(nid) = normal_map_id {
                scene.texture_store.get(nid)
            } else {
                scene.texture_store.get(scene.texture_store.default_normal_id())
            };

            let is_skinned = scene.mesh_store.get(mesh_id).is_some_and(|m| m.is_skinned);
            let joint_data = if is_skinned {
                scene.joint_matrix_cache.get(&eid).copied()
            } else {
                None
            };

            if let Some(cached) = cache.get_mut(&eid) {
                // Cache hit — update buffers via write_buffer (no GPU alloc)
                cached.last_seen_frame = current_frame;
                self.queue.write_buffer(
                    &cached.model_buffer,
                    0,
                    bytemuck::cast_slice(&[model_uniforms]),
                );
                self.queue.write_buffer(
                    &cached.mat_buffer,
                    0,
                    bytemuck::cast_slice(&[mat_uniforms]),
                );

                // Recreate material bind group only if textures changed
                if cached.last_texture_id != texture_id
                    || cached.last_normal_map_id != normal_map_id
                {
                    cached.mat_bg = create_material_bind_group(
                        &self.device,
                        &scene.pipeline.material_bgl,
                        &cached.mat_buffer,
                        gpu_tex,
                        gpu_normal,
                    );
                    cached.last_texture_id = texture_id;
                    cached.last_normal_map_id = normal_map_id;
                }

                // Joint handling
                if is_skinned {
                    if let Some(jd) = joint_data {
                        let joint_uniforms =
                            crate::renderer::skinned_pipeline::JointMatricesUniform {
                                matrices: jd,
                            };
                        if let Some(ref joint_buf) = cached.joint_buffer {
                            self.queue.write_buffer(
                                joint_buf,
                                0,
                                bytemuck::cast_slice(&[joint_uniforms]),
                            );
                        } else {
                            let joint_buffer = self.device.create_buffer_init(
                                &wgpu::util::BufferInitDescriptor {
                                    label: Some("Joint Matrices Uniform"),
                                    contents: bytemuck::cast_slice(&[joint_uniforms]),
                                    usage: buf_usage,
                                },
                            );
                            let joint_bg =
                                crate::renderer::skinned_pipeline::create_joint_bind_group(
                                    &self.device,
                                    &scene.skinned_pipeline.joint_bgl,
                                    &joint_buffer,
                                );
                            cached.joint_buffer = Some(joint_buffer);
                            cached.joint_bg = Some(joint_bg);
                        }
                    }
                } else {
                    // Not skinned — drop joint data if present
                    cached.joint_buffer = None;
                    cached.joint_bg = None;
                }
            } else {
                // Cache miss — create everything with UNIFORM | COPY_DST
                let model_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Model Uniform"),
                            contents: bytemuck::cast_slice(&[model_uniforms]),
                            usage: buf_usage,
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
                            usage: buf_usage,
                        });

                let mat_bg = create_material_bind_group(
                    &self.device,
                    &scene.pipeline.material_bgl,
                    &mat_buffer,
                    gpu_tex,
                    gpu_normal,
                );

                let (joint_buffer, joint_bg) = if is_skinned {
                    if let Some(jd) = joint_data {
                        let joint_uniforms =
                            crate::renderer::skinned_pipeline::JointMatricesUniform {
                                matrices: jd,
                            };
                        let jbuf = self.device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("Joint Matrices Uniform"),
                                contents: bytemuck::cast_slice(&[joint_uniforms]),
                                usage: buf_usage,
                            },
                        );
                        let jbg = crate::renderer::skinned_pipeline::create_joint_bind_group(
                            &self.device,
                            &scene.skinned_pipeline.joint_bgl,
                            &jbuf,
                        );
                        (Some(jbuf), Some(jbg))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                cache.insert(
                    eid,
                    CachedEntityGpu {
                        model_buffer,
                        model_bg,
                        mat_buffer,
                        mat_bg,
                        joint_buffer,
                        joint_bg,
                        last_texture_id: texture_id,
                        last_normal_map_id: normal_map_id,
                        last_seen_frame: current_frame,
                    },
                );
            }
        }

        // Cleanup stale entities (not seen for 2+ frames)
        cache.retain(|_, cached| current_frame.wrapping_sub(cached.last_seen_frame) < 2);

        scene.entity_gpu_cache = cache;

        (renderables, culled)
    }
}
