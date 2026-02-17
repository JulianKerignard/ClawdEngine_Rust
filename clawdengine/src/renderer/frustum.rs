use glam::{Mat4, Vec3, Vec4};

/// Six-plane frustum extracted from a view-projection matrix.
/// Each plane is stored as `Vec4(a, b, c, d)` where `ax + by + cz + d >= 0` means inside.
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// Extract frustum planes from a view-projection matrix using the
    /// Griess-Hartmann method (rows of the VP matrix).
    pub fn from_view_proj(vp: Mat4) -> Self {
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let mut planes = [
            row3 + row0, // Left
            row3 - row0, // Right
            row3 + row1, // Bottom
            row3 - row1, // Top
            row2,        // Near  (wgpu: z in [0,1])
            row3 - row2, // Far
        ];

        // Normalize each plane so the normal (a,b,c) is unit length.
        for p in &mut planes {
            let len = Vec3::new(p.x, p.y, p.z).length();
            if len > 1e-8 {
                *p /= len;
            }
        }

        Self { planes }
    }

    /// Test whether an axis-aligned bounding box intersects this frustum.
    /// Uses the "positive vertex" (p-vertex) test: for each plane, the corner
    /// of the AABB most aligned with the plane normal is checked. If that
    /// corner is behind the plane, the whole AABB is outside.
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for p in &self.planes {
            // Select the positive vertex: for each axis, pick max if normal
            // component is positive, min otherwise.
            let px = if p.x >= 0.0 { max.x } else { min.x };
            let py = if p.y >= 0.0 { max.y } else { min.y };
            let pz = if p.z >= 0.0 { max.z } else { min.z };

            // dot(normal, p_vertex) + d
            if p.x * px + p.y * py + p.z * pz + p.w < 0.0 {
                return false;
            }
        }
        true
    }
}
