mod entity_data;
mod frame;
mod initialization;
mod render_passes;
mod scene_setup;

use crate::core::{EntityId, MAX_JOINTS};

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui_renderer: egui_wgpu::Renderer,
}

pub struct CachedEntityGpu {
    pub model_buffer: wgpu::Buffer,
    pub model_bg: wgpu::BindGroup,
    pub mat_buffer: wgpu::Buffer,
    pub mat_bg: wgpu::BindGroup,
    pub joint_buffer: Option<wgpu::Buffer>,
    pub joint_bg: Option<wgpu::BindGroup>,
    pub last_texture_id: Option<usize>,
    pub last_normal_map_id: Option<usize>,
    pub last_seen_frame: u32,
}

pub struct SceneRenderer {
    pub pipeline: crate::renderer::pipeline::MeshPipeline,
    pub line_pipeline: crate::renderer::line_pipeline::LinePipeline,
    pub line_batch: crate::renderer::line_pipeline::LineBatch,
    pub viewport: crate::renderer::viewport::ViewportTexture,
    pub camera: crate::renderer::camera::Camera,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub lights_buffer: wgpu::Buffer,
    pub lights_bind_group: wgpu::BindGroup,
    pub mesh_store: crate::renderer::mesh::MeshStore,
    pub texture_store: crate::renderer::texture_store::TextureStore,
    pub skybox: crate::renderer::skybox::SkyboxPipeline,
    pub shadow_map: crate::renderer::shadow::ShadowMap,
    pub skinned_pipeline: crate::renderer::skinned_pipeline::SkinnedMeshPipeline,
    pub skinned_shadow_pipeline: crate::renderer::skinned_pipeline::SkinnedShadowPipeline,
    pub skeleton_store: crate::core::SkeletonStore,
    pub animation_clip_store: crate::core::AnimationClipStore,
    pub animator_controller_store: crate::core::AnimatorControllerStore,
    pub joint_matrix_cache: std::collections::HashMap<EntityId, [[[f32; 4]; 4]; MAX_JOINTS]>,
    pub game_viewport: Option<crate::renderer::viewport::ViewportTexture>,
    pub game_camera_buffer: wgpu::Buffer,
    pub game_camera_bind_group: wgpu::BindGroup,
    pub entity_gpu_cache: std::collections::HashMap<EntityId, CachedEntityGpu>,
    pub frame_counter: u32,
    pub anim_local_poses_buf: Vec<crate::core::Transform>,
    pub anim_global_buf: Vec<glam::Mat4>,
    pub anim_joint_buf: Vec<glam::Mat4>,
    pub anim_results_buf: Vec<(EntityId, [[[f32; 4]; 4]; MAX_JOINTS])>,
}
