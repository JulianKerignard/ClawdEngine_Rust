use wgpu::util::DeviceExt;

use crate::renderer::camera::Camera;
use crate::renderer::line_pipeline::{LineBatch, LinePipeline};
use crate::renderer::mesh::MeshStore;
use crate::renderer::pipeline::{LightData, LightsUniforms, MeshPipeline};
use crate::renderer::shadow::ShadowMap;
use crate::renderer::skinned_pipeline::{SkinnedMeshPipeline, SkinnedShadowPipeline};
use crate::renderer::skybox::SkyboxPipeline;
use crate::renderer::texture_store::TextureStore;
use crate::renderer::viewport::ViewportTexture;

use super::{GpuContext, SceneRenderer};

fn create_camera_setup(
    device: &wgpu::Device,
    camera_bgl: &wgpu::BindGroupLayout,
    uniforms: &crate::renderer::camera::CameraUniforms,
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
        let shadow_bgl = ShadowMap::create_shadow_bgl(&gpu.device);
        let pipeline = MeshPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &shadow_bgl);
        let shadow_map = ShadowMap::new(&gpu.device, &pipeline.model_bgl, shadow_bgl);

        let shadow_bgl_for_skinned = ShadowMap::create_shadow_bgl(&gpu.device);
        let skinned_pipeline = SkinnedMeshPipeline::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            &pipeline.camera_bgl,
            &pipeline.model_bgl,
            &pipeline.material_bgl,
            &pipeline.lights_bgl,
            &shadow_bgl_for_skinned,
        );
        let skinned_shadow_pipeline = SkinnedShadowPipeline::new(
            &gpu.device,
            &shadow_map.light_vp_bgl,
            &pipeline.model_bgl,
        );

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
            skinned_pipeline,
            skinned_shadow_pipeline,
            skeleton_store: crate::core::SkeletonStore::new(),
            animation_clip_store: crate::core::AnimationClipStore::new(),
            joint_matrix_cache: std::collections::HashMap::new(),
            game_viewport: None,
            game_camera_buffer,
            game_camera_bind_group,
            entity_gpu_cache: std::collections::HashMap::new(),
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
