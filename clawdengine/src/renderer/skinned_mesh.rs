#![allow(dead_code)]
use bytemuck::{Pod, Zeroable};

/// Vertex with skinning data (joints + weights) for skeletal animation.
/// 72 bytes total, extends Vertex with 2 extra attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4],
    pub joint_indices: [u16; 4],
    pub joint_weights: [f32; 4],
}

impl SkinnedVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkinnedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,   // position
            1 => Float32x3,   // normal
            2 => Float32x2,   // uv
            3 => Float32x4,   // tangent
            4 => Uint16x4,    // joint_indices
            5 => Float32x4,   // joint_weights
        ],
    };
}

// Verify size at compile time via const assertion
const _: () = assert!(std::mem::size_of::<SkinnedVertex>() == 72);

/// GPU-side skinned mesh (stub for tasks #76/#80).
pub struct SkinnedGpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub skeleton_id: Option<usize>,
}
