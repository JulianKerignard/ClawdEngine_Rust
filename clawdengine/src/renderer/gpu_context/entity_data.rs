use wgpu::util::DeviceExt;

use crate::core::{EntityId, World};
use crate::renderer::pipeline::{MaterialUniforms, ModelUniforms};
use crate::renderer::scene_helpers::SELECTION_TINT;

use super::{GpuContext, SceneRenderer};

impl GpuContext {
    pub(crate) fn prepare_entity_data<'a>(
        &self,
        scene: &'a SceneRenderer,
        world: &'a World,
        selected_entity: Option<EntityId>,
    ) -> (
        Vec<(EntityId, &'a crate::core::Transform, usize, Option<&'a crate::core::Material>)>,
        Vec<(wgpu::BindGroup, wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, Option<wgpu::BindGroup>)>,
    ) {
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

        let per_entity_data: Vec<_> = renderables
            .iter()
            .map(|(eid, transform, mesh_id, material)| {
                let wt = world.get_world_transform(*eid).unwrap_or(**transform);
                let model_matrix = glam::Mat4::from_scale_rotation_translation(
                    wt.scale,
                    wt.rotation,
                    wt.position,
                );

                let color = if selected_entity == Some(*eid) {
                    SELECTION_TINT
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };

                let model_uniforms = ModelUniforms {
                    model: model_matrix.to_cols_array_2d(),
                    color,
                };

                let model_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Model Uniform"),
                            contents: bytemuck::cast_slice(&[model_uniforms]),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                let model_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Model BG"),
                    layout: &scene.pipeline.model_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: model_buffer.as_entire_binding(),
                    }],
                });

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

                let mat_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Material Uniform"),
                            contents: bytemuck::cast_slice(&[mat_uniforms]),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                let gpu_tex = if let Some(tex_id) = material.and_then(|m| m.texture_id) {
                    scene.texture_store.get(tex_id)
                } else {
                    scene.texture_store.get(scene.texture_store.default_id())
                };

                let gpu_normal = if let Some(nid) = material.and_then(|m| m.normal_map_id) {
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

                let joint_bg = if scene.mesh_store.get(*mesh_id).map_or(false, |m| m.is_skinned) {
                    if let Some(joint_data) = scene.joint_matrix_cache.get(eid) {
                        let joint_uniforms = crate::renderer::skinned_pipeline::JointMatricesUniform {
                            matrices: *joint_data,
                        };
                        let joint_buffer =
                            self.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("Joint Matrices Uniform"),
                                    contents: bytemuck::cast_slice(&[joint_uniforms]),
                                    usage: wgpu::BufferUsages::UNIFORM,
                                });
                        Some(crate::renderer::skinned_pipeline::create_joint_bind_group(
                            &self.device,
                            &scene.skinned_pipeline.joint_bgl,
                            &joint_buffer,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };

                (model_bg, mat_bg, model_buffer, mat_buffer, joint_bg)
            })
            .collect();

        (renderables, per_entity_data)
    }
}
