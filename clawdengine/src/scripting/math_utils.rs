#![allow(dead_code)]
use glam::{Vec3, Quat, Mat3};

/// Simple pseudo-random number generator (xorshift32). Not cryptographic.
pub struct GameRng {
    state: u32,
}

impl GameRng {
    pub fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }

    /// Random f32 in [0.0, 1.0).
    pub fn random_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Random f32 in [min, max).
    pub fn random_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.random_f32() * (max - min)
    }
}

/// Linear interpolation between two f32 values.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear interpolation between two Vec3 values.
pub fn lerp_vec3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// Distance between two Vec3 points.
pub fn distance(a: Vec3, b: Vec3) -> f32 {
    (b - a).length()
}

/// Squared distance (avoids sqrt, useful for comparisons).
pub fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    (b - a).length_squared()
}

/// Compute a rotation quaternion that looks from `from` toward `target` (Y-up).
pub fn look_at_rotation(from: Vec3, target: Vec3) -> Quat {
    let dir = (target - from).normalize_or_zero();
    if dir.length_squared() < 1e-6 {
        return Quat::IDENTITY;
    }
    // Looking along -Z in right-handed coordinates
    let forward = -dir;
    let up = Vec3::Y;
    let right = up.cross(forward).normalize_or_zero();
    if right.length_squared() < 1e-6 {
        // Looking straight up or down
        return Quat::from_rotation_x(
            if dir.y > 0.0 { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 },
        );
    }
    let corrected_up = forward.cross(right).normalize();
    Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward))
}

/// Move a value toward a target by at most `max_step`.
pub fn move_toward(current: f32, target: f32, max_step: f32) -> f32 {
    if (target - current).abs() <= max_step {
        target
    } else {
        current + (target - current).signum() * max_step
    }
}
