use crate::core::EntityId;

use super::{GpuContext, SceneRenderer};

impl GpuContext {
    pub(crate) fn execute_shadow_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneRenderer,
        renderables: &[(EntityId, &crate::core::Transform, usize, Option<&crate::core::Material>)],
        per_entity_data: &[(wgpu::BindGroup, wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, Option<wgpu::BindGroup>)],
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

        // Normal meshes
        shadow_pass.set_pipeline(&scene.shadow_map.pipeline);
        shadow_pass.set_bind_group(0, &scene.shadow_map.light_vp_bind_group, &[]);
        for (i, (_eid, _transform, mesh_id, _material)) in renderables.iter().enumerate() {
            if per_entity_data[i].4.is_some() { continue; }
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                let (ref model_bg, _, _, _, _) = per_entity_data[i];
                shadow_pass.set_bind_group(1, model_bg, &[]);
                shadow_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }

        // Skinned meshes
        shadow_pass.set_pipeline(&scene.skinned_shadow_pipeline.pipeline);
        shadow_pass.set_bind_group(0, &scene.shadow_map.light_vp_bind_group, &[]);
        for (i, (_eid, _transform, mesh_id, _material)) in renderables.iter().enumerate() {
            let Some(ref joint_bg) = per_entity_data[i].4 else { continue; };
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                let (ref model_bg, _, _, _, _) = per_entity_data[i];
                shadow_pass.set_bind_group(1, model_bg, &[]);
                shadow_pass.set_bind_group(2, joint_bg, &[]);
                shadow_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
        }
    }

    pub(crate) fn execute_main_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneRenderer,
        camera_bg: &wgpu::BindGroup,
        msaa_view: &wgpu::TextureView,
        resolve_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        renderables: &[(EntityId, &crate::core::Transform, usize, Option<&crate::core::Material>)],
        per_entity_data: &[(wgpu::BindGroup, wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, Option<wgpu::BindGroup>)],
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

        // Normal meshes
        pass.set_pipeline(&scene.pipeline.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);
        pass.set_bind_group(3, &scene.lights_bind_group, &[]);
        pass.set_bind_group(4, &scene.shadow_map.shadow_bind_group, &[]);
        for (i, (_eid, _transform, mesh_id, _material)) in renderables.iter().enumerate() {
            if per_entity_data[i].4.is_some() { continue; }
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                let (ref model_bg, ref mat_bg, _, _, _) = per_entity_data[i];
                pass.set_bind_group(1, model_bg, &[]);
                pass.set_bind_group(2, mat_bg, &[]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
                draw_call_count += 1;
                triangle_count += gpu_mesh.index_count / 3;
            }
        }

        // Skinned meshes
        pass.set_pipeline(&scene.skinned_pipeline.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);
        pass.set_bind_group(3, &scene.lights_bind_group, &[]);
        pass.set_bind_group(4, &scene.shadow_map.shadow_bind_group, &[]);
        for (i, (_eid, _transform, mesh_id, _material)) in renderables.iter().enumerate() {
            let Some(ref joint_bg) = per_entity_data[i].4 else { continue; };
            if let Some(gpu_mesh) = scene.mesh_store.get(*mesh_id) {
                let (ref model_bg, ref mat_bg, _, _, _) = per_entity_data[i];
                pass.set_bind_group(1, model_bg, &[]);
                pass.set_bind_group(2, mat_bg, &[]);
                pass.set_bind_group(5, joint_bg, &[]);
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
}
