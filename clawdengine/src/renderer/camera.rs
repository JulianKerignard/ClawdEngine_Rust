use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub eye_position: [f32; 4],
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub up: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let eye = Vec3::new(3.0, 3.0, 3.0);
        let target = Vec3::ZERO;
        let offset = eye - target;
        let distance = offset.length();
        let yaw = offset.z.atan2(offset.x);
        let pitch = (offset.y / distance).asin();

        Self {
            position: eye,
            yaw,
            pitch,
            up: Vec3::Y,
            fov_y: 45.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,
        }
    }
}

impl Camera {
    /// Direction the camera is looking (unit vector)
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            -self.pitch.cos() * self.yaw.cos(),
            -self.pitch.sin(),
            -self.pitch.cos() * self.yaw.sin(),
        )
        .normalize()
    }

    pub fn eye(&self) -> Vec3 {
        self.position
    }

    pub fn target(&self) -> Vec3 {
        self.position + self.forward()
    }

    pub fn uniforms(&self, aspect: f32) -> CameraUniforms {
        let eye = self.position;
        let view = Mat4::look_at_rh(eye, self.target(), self.up);
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
        CameraUniforms {
            view_proj: (proj * view).to_cols_array_2d(),
            eye_position: [eye.x, eye.y, eye.z, 1.0],
        }
    }

    /// Rotate the camera in place (free look)
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.005;
        self.yaw += delta_x * sensitivity;
        self.pitch += delta_y * sensitivity;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    /// Move camera along its forward axis (scroll zoom)
    pub fn zoom(&mut self, delta: f32) {
        let speed = 0.5;
        self.position += self.forward() * delta * speed;
    }

    /// Pan camera (translate in screen plane)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let fwd = self.forward();
        let right = fwd.cross(self.up).normalize();
        let up = right.cross(fwd).normalize();

        let sensitivity = 0.005;
        self.position += right * (-delta_x * sensitivity) + up * (delta_y * sensitivity);
    }

    /// WASD fly movement in camera-relative directions
    pub fn fly(&mut self, forward: f32, right: f32, up: f32) {
        let fwd = self.forward();
        let r = fwd.cross(self.up).normalize();
        self.position += fwd * forward + r * right + Vec3::Y * up;
    }

    /// Focus on a position (move eye to look at it from current distance)
    pub fn focus_on(&mut self, target: Vec3) {
        let offset = self.position - target;
        let distance = offset.length().max(2.0);
        // Recompute yaw/pitch to look at target
        self.yaw = offset.z.atan2(offset.x);
        self.pitch = (offset.y / distance).asin();
        // Place eye at a comfortable distance
        self.position = target + Vec3::new(
            distance * self.pitch.cos() * self.yaw.cos(),
            distance * self.pitch.sin(),
            distance * self.pitch.cos() * self.yaw.sin(),
        );
    }

    /// Convert viewport-relative screen coords to a world-space ray (origin, direction).
    pub fn screen_to_ray(&self, sx: f32, sy: f32, vp_w: f32, vp_h: f32) -> (Vec3, Vec3) {
        let ndc_x = 2.0 * sx / vp_w - 1.0;
        let ndc_y = 1.0 - 2.0 * sy / vp_h;

        let aspect = vp_w / vp_h;
        let eye = self.position;
        let view = Mat4::look_at_rh(eye, self.target(), self.up);
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
        let inv_vp = (proj * view).inverse();

        // wgpu z-range: near=0, far=1
        let near_clip = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far_clip = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        let dir = (far_clip - near_clip).normalize();
        (near_clip, dir)
    }
}
