use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4], // xyz = tangent direction, w = handedness (+1 or -1)
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Float32x4,
        ],
    };
}

#[derive(Clone, Copy, Debug)]
pub struct MeshAABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl MeshAABB {
    pub fn from_vertices(vertices: &[Vertex]) -> Self {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in vertices {
            min = min.min(Vec3::from(v.position));
            max = max.max(Vec3::from(v.position));
        }
        Self { min, max }
    }

    /// Transform AABB by model matrix (recompute from 8 corners).
    pub fn transformed(&self, model: Mat4) -> (Vec3, Vec3) {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for c in &corners {
            let t = model.transform_point3(*c);
            min = min.min(t);
            max = max.max(t);
        }
        (min, max)
    }
}

pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub struct MeshStore {
    meshes: Vec<GpuMesh>,
    aabbs: Vec<MeshAABB>,
    names: Vec<String>,
}

impl MeshStore {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            aabbs: Vec::new(),
            names: Vec::new(),
        }
    }

    pub fn add_named(&mut self, device: &wgpu::Device, vertices: &[Vertex], indices: &[u32], name: &str) -> usize {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let aabb = MeshAABB::from_vertices(vertices);
        let id = self.meshes.len();
        self.meshes.push(GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        });
        self.aabbs.push(aabb);
        self.names.push(name.to_string());
        id
    }

    #[allow(dead_code)]
    pub fn add(&mut self, device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> usize {
        self.add_named(device, vertices, indices, "")
    }

    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    pub fn get(&self, id: usize) -> Option<&GpuMesh> {
        self.meshes.get(id)
    }

    pub fn get_aabb(&self, id: usize) -> Option<&MeshAABB> {
        self.aabbs.get(id)
    }

    pub fn len(&self) -> usize {
        self.meshes.len()
    }
}

// ---- Procedural cube ----

pub fn generate_cube() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    // Each face: 4 vertices with face normal, 2 triangles (6 indices)
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        // +X
        ([1.0, 0.0, 0.0], [
            [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5],
        ]),
        // -X
        ([-1.0, 0.0, 0.0], [
            [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5],
        ]),
        // +Y
        ([0.0, 1.0, 0.0], [
            [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
        ]),
        // -Y
        ([0.0, -1.0, 0.0], [
            [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5],
        ]),
        // +Z
        ([0.0, 0.0, 1.0], [
            [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5],
        ]),
        // -Z
        ([0.0, 0.0, -1.0], [
            [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5],
        ]),
    ];

    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    for (normal, positions) in &faces {
        let base = vertices.len() as u32;
        for (i, pos) in positions.iter().enumerate() {
            vertices.push(Vertex {
                position: *pos,
                normal: *normal,
                uv: uvs[i],
                tangent: [0.0; 4],
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

// ---- Procedural UV sphere ----

pub fn generate_sphere(rings: u32, sectors: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(((rings + 1) * (sectors + 1)) as usize);
    let mut indices = Vec::with_capacity((rings * sectors * 6) as usize);

    for r in 0..=rings {
        let theta = std::f32::consts::PI * r as f32 / rings as f32;
        let sin_t = theta.sin();
        let cos_t = theta.cos();

        for s in 0..=sectors {
            let phi = 2.0 * std::f32::consts::PI * s as f32 / sectors as f32;
            let x = sin_t * phi.cos();
            let y = cos_t;
            let z = sin_t * phi.sin();

            vertices.push(Vertex {
                position: [x * 0.5, y * 0.5, z * 0.5],
                normal: [x, y, z],
                uv: [s as f32 / sectors as f32, r as f32 / rings as f32],
                tangent: [0.0; 4],
            });
        }
    }

    let row_len = sectors + 1;
    for r in 0..rings {
        for s in 0..sectors {
            let cur = r * row_len + s;
            let next = cur + row_len;
            indices.extend_from_slice(&[cur, next, cur + 1, cur + 1, next, next + 1]);
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// MikkTSpace-style tangent computation from triangle geometry.
pub fn compute_tangents(vertices: &mut [Vertex], indices: &[u32]) {
    let vc = vertices.len();
    let mut tan_acc = vec![[0.0f32; 3]; vc];
    let mut bitan_acc = vec![[0.0f32; 3]; vc];

    for tri in indices.chunks(3) {
        if tri.len() < 3 { break; }
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = Vec3::from(vertices[i0].position);
        let p1 = Vec3::from(vertices[i1].position);
        let p2 = Vec3::from(vertices[i2].position);
        let uv0 = vertices[i0].uv;
        let uv1 = vertices[i1].uv;
        let uv2 = vertices[i2].uv;

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let duv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
        let duv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];

        let det = duv1[0] * duv2[1] - duv1[1] * duv2[0];
        if det.abs() < 1e-8 { continue; }
        let inv = 1.0 / det;

        let t = (edge1 * duv2[1] - edge2 * duv1[1]) * inv;
        let b = (edge2 * duv1[0] - edge1 * duv2[0]) * inv;

        for &idx in &[i0, i1, i2] {
            tan_acc[idx][0] += t.x;
            tan_acc[idx][1] += t.y;
            tan_acc[idx][2] += t.z;
            bitan_acc[idx][0] += b.x;
            bitan_acc[idx][1] += b.y;
            bitan_acc[idx][2] += b.z;
        }
    }

    for i in 0..vc {
        let n = Vec3::from(vertices[i].normal);
        let t = Vec3::from(tan_acc[i]);
        let b = Vec3::from(bitan_acc[i]);
        // Gram-Schmidt orthogonalize
        let t_ortho = (t - n * n.dot(t)).normalize_or_zero();
        // Handedness
        let w = if n.cross(t_ortho).dot(b) < 0.0 { -1.0 } else { 1.0 };
        vertices[i].tangent = [t_ortho.x, t_ortho.y, t_ortho.z, w];
    }
}
