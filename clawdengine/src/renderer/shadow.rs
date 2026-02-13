use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::mesh::Vertex;

pub const SHADOW_MAP_SIZE: u32 = 2048;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LightVPUniforms {
    pub view_proj: [[f32; 4]; 4],
}

pub struct ShadowMap {
    pub depth_view: wgpu::TextureView,
    pub light_vp_buffer: wgpu::Buffer,
    pub light_vp_bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    pub shadow_bind_group: wgpu::BindGroup,
    _depth_texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
}

impl ShadowMap {
    /// Create just the shadow BGL (needed before MeshPipeline creation).
    pub fn create_shadow_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn new(
        device: &wgpu::Device,
        model_bgl: &wgpu::BindGroupLayout,
        shadow_bgl: wgpu::BindGroupLayout,
    ) -> Self {
        let size = SHADOW_MAP_SIZE;

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Depth"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let light_vp_uniforms = LightVPUniforms {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let light_vp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VP Buffer"),
            contents: bytemuck::cast_slice(&[light_vp_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // BGL for shadow depth pass (group 0 = light VP)
        let light_vp_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light VP BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let light_vp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light VP BG"),
            layout: &light_vp_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_vp_buffer.as_entire_binding(),
            }],
        });

        // Bind group for main mesh pass (group 4)
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow BG"),
            layout: &shadow_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_vp_buffer.as_entire_binding(),
                },
            ],
        });

        // Shadow depth pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/shadow.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[&light_vp_bgl, model_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Vertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview: None,
            cache: None,
        });

        Self {
            depth_view,
            light_vp_buffer,
            light_vp_bind_group,
            pipeline,
            shadow_bind_group,
            _depth_texture: depth_texture,
            _sampler: sampler,
        }
    }

    pub fn compute_light_vp(light_dir: Vec3) -> Mat4 {
        let dir = light_dir.normalize();
        let light_pos = -dir * 20.0;
        let view = Mat4::look_at_rh(light_pos, Vec3::ZERO, Vec3::Y);
        let half = 15.0;
        let proj = Mat4::orthographic_rh(-half, half, -half, half, 0.1, 50.0);
        proj * view
    }

    pub fn update_light_vp(&self, queue: &wgpu::Queue, light_vp: Mat4) {
        let uniforms = LightVPUniforms {
            view_proj: light_vp.to_cols_array_2d(),
        };
        queue.write_buffer(
            &self.light_vp_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }
}
