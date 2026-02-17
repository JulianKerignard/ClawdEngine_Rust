use bytemuck::{Pod, Zeroable};

// ---- Line Vertex ----

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x4,
        ],
    };
}

// ---- Helpers ----

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-8 {
        return [0.0, 1.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

// ---- Line Batch ----

const INITIAL_THIN_CAP: u32 = 4096;
const INITIAL_THICK_CAP: u32 = 1024;

fn create_buffer(device: &wgpu::Device, label: &str, cap: u32) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (cap as usize * std::mem::size_of::<LineVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub struct LineBatch {
    // Thin lines (1px, LineList topology)
    thin_verts: Vec<LineVertex>,
    thin_buffer: wgpu::Buffer,
    thin_cap: u32,
    // Thick lines (billboard quads, TriangleList topology)
    thick_verts: Vec<LineVertex>,
    thick_buffer: wgpu::Buffer,
    thick_cap: u32,
}

impl LineBatch {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            thin_verts: Vec::with_capacity(INITIAL_THIN_CAP as usize),
            thin_buffer: create_buffer(device, "Thin Line Buffer", INITIAL_THIN_CAP),
            thin_cap: INITIAL_THIN_CAP,
            thick_verts: Vec::with_capacity(INITIAL_THICK_CAP as usize),
            thick_buffer: create_buffer(device, "Thick Line Buffer", INITIAL_THICK_CAP),
            thick_cap: INITIAL_THICK_CAP,
        }
    }

    pub fn clear(&mut self) {
        self.thin_verts.clear();
        self.thick_verts.clear();
    }

    /// Push 12 thin lines forming a wireframe AABB box.
    pub fn push_aabb_wireframe(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 4]) {
        let c = [
            [min[0], min[1], min[2]], // 0: bottom-front-left
            [max[0], min[1], min[2]], // 1: bottom-front-right
            [max[0], max[1], min[2]], // 2: top-front-right
            [min[0], max[1], min[2]], // 3: top-front-left
            [min[0], min[1], max[2]], // 4: bottom-back-left
            [max[0], min[1], max[2]], // 5: bottom-back-right
            [max[0], max[1], max[2]], // 6: top-back-right
            [min[0], max[1], max[2]], // 7: top-back-left
        ];
        // Bottom face
        self.push_line(c[0], c[1], color);
        self.push_line(c[1], c[5], color);
        self.push_line(c[5], c[4], color);
        self.push_line(c[4], c[0], color);
        // Top face
        self.push_line(c[3], c[2], color);
        self.push_line(c[2], c[6], color);
        self.push_line(c[6], c[7], color);
        self.push_line(c[7], c[3], color);
        // Vertical edges
        self.push_line(c[0], c[3], color);
        self.push_line(c[1], c[2], color);
        self.push_line(c[5], c[6], color);
        self.push_line(c[4], c[7], color);
    }

    /// Push a 1px thin line (LineList).
    pub fn push_line(&mut self, a: [f32; 3], b: [f32; 3], color: [f32; 4]) {
        self.thin_verts.push(LineVertex { position: a, color });
        self.thin_verts.push(LineVertex { position: b, color });
    }

    /// Push a dashed thin line (multiple short segments with gaps).
    pub fn push_dashed_line(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        color: [f32; 4],
        dash_len: f32,
        gap_len: f32,
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let dz = b[2] - a[2];
        let total_len = (dx * dx + dy * dy + dz * dz).sqrt();
        if total_len < 0.001 {
            return;
        }
        let dir = [dx / total_len, dy / total_len, dz / total_len];
        let step = dash_len + gap_len;
        let mut t = 0.0_f32;
        while t < total_len {
            let t_end = (t + dash_len).min(total_len);
            let start = [
                a[0] + dir[0] * t,
                a[1] + dir[1] * t,
                a[2] + dir[2] * t,
            ];
            let end = [
                a[0] + dir[0] * t_end,
                a[1] + dir[1] * t_end,
                a[2] + dir[2] * t_end,
            ];
            self.push_line(start, end, color);
            t += step;
        }
    }

    /// Push a thick line as a camera-facing quad (2 triangles, 6 vertices).
    pub fn push_billboard_line(
        &mut self,
        a: [f32; 3],
        b: [f32; 3],
        color: [f32; 4],
        width: f32,
        camera_eye: [f32; 3],
    ) {
        let line_dir = normalize(sub(b, a));
        let mid = [
            (a[0] + b[0]) * 0.5,
            (a[1] + b[1]) * 0.5,
            (a[2] + b[2]) * 0.5,
        ];
        let cam_dir = normalize(sub(camera_eye, mid));
        let side = normalize(cross(line_dir, cam_dir));
        let half = scale(side, width * 0.5);

        // 4 corners of the quad
        let a0 = add(a, half);
        let a1 = sub(a, half);
        let b0 = add(b, half);
        let b1 = sub(b, half);

        // Triangle 1: a0, a1, b0
        self.thick_verts.push(LineVertex { position: a0, color });
        self.thick_verts.push(LineVertex { position: a1, color });
        self.thick_verts.push(LineVertex { position: b0, color });
        // Triangle 2: a1, b1, b0
        self.thick_verts.push(LineVertex { position: a1, color });
        self.thick_verts.push(LineVertex { position: b1, color });
        self.thick_verts.push(LineVertex { position: b0, color });
    }

    /// Push a cone (arrow tip) as a triangle fan. `tip` is the apex, `base_center`
    /// is the center of the circular base, `radius` is the base radius.
    pub fn push_cone(
        &mut self,
        tip: [f32; 3],
        base_center: [f32; 3],
        radius: f32,
        color: [f32; 4],
        segments: u32,
    ) {
        let dir = normalize(sub(tip, base_center));

        // Find two perpendicular vectors to dir
        let up = if dir[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let perp1 = normalize(cross(dir, up));
        let perp2 = normalize(cross(dir, perp1));

        let step = std::f32::consts::TAU / segments as f32;

        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;

            let (s0, c0) = (a0.sin(), a0.cos());
            let (s1, c1) = (a1.sin(), a1.cos());

            // Points on the base circle
            let p0 = add(
                base_center,
                add(scale(perp1, c0 * radius), scale(perp2, s0 * radius)),
            );
            let p1 = add(
                base_center,
                add(scale(perp1, c1 * radius), scale(perp2, s1 * radius)),
            );

            // Side triangle: tip, p0, p1
            self.thick_verts.push(LineVertex { position: tip, color });
            self.thick_verts.push(LineVertex { position: p0, color });
            self.thick_verts.push(LineVertex { position: p1, color });

            // Base triangle: base_center, p1, p0 (reversed winding for bottom cap)
            self.thick_verts.push(LineVertex { position: base_center, color });
            self.thick_verts.push(LineVertex { position: p1, color });
            self.thick_verts.push(LineVertex { position: p0, color });
        }
    }

    /// Draw a circle arc as thick billboard segments.
    #[allow(clippy::too_many_arguments)]
    pub fn push_circle_arc(
        &mut self,
        center: [f32; 3],
        axis: [f32; 3],
        radius: f32,
        color: [f32; 4],
        segments: u32,
        width: f32,
        camera_eye: [f32; 3],
    ) {
        let dir = normalize(axis);
        let up = if dir[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let perp1 = normalize(cross(dir, up));
        let perp2 = normalize(cross(dir, perp1));

        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            let pa = add(
                center,
                add(scale(perp1, a0.cos() * radius), scale(perp2, a0.sin() * radius)),
            );
            let pb = add(
                center,
                add(scale(perp1, a1.cos() * radius), scale(perp2, a1.sin() * radius)),
            );
            self.push_billboard_line(pa, pb, color, width, camera_eye);
        }
    }

    /// Draw a small solid marker (3 crossed billboard quads) simulating a cube.
    pub fn push_box_marker(
        &mut self,
        center: [f32; 3],
        half_size: f32,
        color: [f32; 4],
        camera_eye: [f32; 3],
    ) {
        let h = half_size;
        self.push_billboard_line(
            [center[0] - h, center[1], center[2]],
            [center[0] + h, center[1], center[2]],
            color, h, camera_eye,
        );
        self.push_billboard_line(
            [center[0], center[1] - h, center[2]],
            [center[0], center[1] + h, center[2]],
            color, h, camera_eye,
        );
        self.push_billboard_line(
            [center[0], center[1], center[2] - h],
            [center[0], center[1], center[2] + h],
            color, h, camera_eye,
        );
    }

    pub fn thin_count(&self) -> u32 {
        self.thin_verts.len() as u32
    }

    pub fn thick_count(&self) -> u32 {
        self.thick_verts.len() as u32
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Thin lines
        if !self.thin_verts.is_empty() {
            let needed = self.thin_verts.len() as u32;
            if needed > self.thin_cap {
                self.thin_cap = needed.next_power_of_two();
                self.thin_buffer = create_buffer(device, "Thin Line Buffer", self.thin_cap);
            }
            queue.write_buffer(&self.thin_buffer, 0, bytemuck::cast_slice(&self.thin_verts));
        }
        // Thick lines
        if !self.thick_verts.is_empty() {
            let needed = self.thick_verts.len() as u32;
            if needed > self.thick_cap {
                self.thick_cap = needed.next_power_of_two();
                self.thick_buffer = create_buffer(device, "Thick Line Buffer", self.thick_cap);
            }
            queue.write_buffer(
                &self.thick_buffer,
                0,
                bytemuck::cast_slice(&self.thick_verts),
            );
        }
    }

    pub fn thin_buffer(&self) -> &wgpu::Buffer {
        &self.thin_buffer
    }

    pub fn thick_buffer(&self) -> &wgpu::Buffer {
        &self.thick_buffer
    }
}

// ---- Line Pipeline ----

pub struct LinePipeline {
    /// 1px lines (LineList topology) — for grid, debug
    pub thin: wgpu::RenderPipeline,
    /// Thick lines as billboard quads (TriangleList topology) — for gizmos
    pub thick: wgpu::RenderPipeline,
}

impl LinePipeline {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/lines.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[camera_bgl],
            push_constant_ranges: &[],
        });

        let depth_state = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let fragment = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        let thin = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Thin Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[LineVertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(fragment.clone()),
            multiview: None,
            cache: None,
        });

        let thick = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Thick Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[LineVertex::LAYOUT],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(depth_state),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(fragment),
            multiview: None,
            cache: None,
        });

        Self { thin, thick }
    }
}
