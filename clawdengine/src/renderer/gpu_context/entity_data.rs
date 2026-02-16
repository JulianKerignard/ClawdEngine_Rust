use wgpu::util::DeviceExt;

use crate::core::{EntityId, World};
use crate::renderer::pipeline::{MaterialUniforms, ModelUniforms};
use crate::renderer::scene_helpers::SELECTION_TINT;

use super::{CachedEntityGpu, GpuContext, SceneRenderer};

impl GpuContext {
    pub(crate) fn prepare_entity_data<'w>(
        &self,
        scene: &mut SceneRenderer,
        world: &'w World,
        selected_entity: Option<EntityId>,
    ) -> Vec<(EntityId, &'w crate::core::Transform, usize, Option<&'w crate::core::Material>)>
    {
        let renderables: Vec<_> = world
            .iter_entities()
            .filter_map(|eid| {
                let transform = world.get_transform(eid)?;
                let mesh_renderer = world.get_mesh_renderer(eid)?;
                if !mesh_renderer.visible {
                    return None;
                }
                let mesh_id = mesh_renderer.mesh_id?;
                let material = world.get_material(eid);
                Some((eid, transform, mesh_id, material))
            })
            .collect();

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

            let is_skinned = scene.mesh_store.get(mesh_id).map_or(false, |m| m.is_skinned);
            let joint_data = if is_skinned {
                scene.joint_matrix_cache.get(&eid).copied()
            } else {
                None
            };

            if let Some(cached) = cache.get_mut(&eid) {
                // Cache hit — update buffers via write_buffer (no GPU alloc)
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
                    cached.mat_bg =
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Material BG"),
                            layout: &scene.pipeline.material_bgl,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: cached.mat_buffer.as_entire_binding(),
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
                                    resource: wgpu::BindingResource::TextureView(
                                        &gpu_normal.view,
                                    ),
                                },
                            ],
                        });
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
                    },
                );
            }
        }

        // Cleanup dead entities
        let active: std::collections::HashSet<EntityId> =
            renderables.iter().map(|(eid, ..)| *eid).collect();
        cache.retain(|eid, _| active.contains(eid));

        scene.entity_gpu_cache = cache;

        renderables
    }
}
