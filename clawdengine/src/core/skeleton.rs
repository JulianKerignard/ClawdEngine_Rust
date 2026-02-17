use glam::Mat4;
use serde::{Serialize, Deserialize};

use super::animator_controller::AnimatorControllerState;
use super::components::Transform;

pub const MAX_JOINTS: usize = 128;

// ---- Bone & Skeleton ----

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    /// Index of parent bone in Skeleton.bones. None = root.
    pub parent: Option<usize>,
    /// Indices of child bones in Skeleton.bones.
    pub children: Vec<usize>,
    /// Inverse of the global bind-pose transform (from glTF inverseBindMatrices).
    pub inverse_bind_matrix: Mat4,
    /// Local transform in bind pose (rest position).
    pub local_bind_transform: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skeleton {
    pub name: String,
    /// Bones in topological order: parent.index < child.index (always).
    pub bones: Vec<Bone>,
    /// Index of root bone in `bones`.
    pub root_bone: usize,
}

impl Skeleton {
    pub fn new(name: String, bones: Vec<Bone>, root_bone: usize) -> Self {
        // Verify topological ordering in debug builds
        debug_assert!(
            bones.iter().enumerate().all(|(i, b)| b.parent.is_none_or(|p| p < i)),
            "Skeleton bones must be topologically sorted (parent index < child index)"
        );
        debug_assert!(root_bone < bones.len(), "root_bone out of bounds");
        Self { name, bones, root_bone }
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn find_bone(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }
}

// ---- SkeletonStore (shared asset, like MeshStore) ----

pub struct SkeletonStore {
    skeletons: Vec<Skeleton>,
    names: Vec<String>,
}

impl SkeletonStore {
    pub fn new() -> Self {
        Self { skeletons: Vec::new(), names: Vec::new() }
    }

    pub fn add(&mut self, skeleton: Skeleton) -> usize {
        let id = self.skeletons.len();
        self.names.push(skeleton.name.clone());
        self.skeletons.push(skeleton);
        id
    }

    pub fn get(&self, id: usize) -> Option<&Skeleton> {
        self.skeletons.get(id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.skeletons.len()
    }
}

// ---- Animation data ----

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InterpolationMode {
    Step,
    Linear,
    CubicSpline,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AnimationProperty {
    Translation,
    Rotation,
    Scale,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationChannel {
    /// Target bone index in the Skeleton.
    pub target_bone: usize,
    /// Animated property.
    pub property: AnimationProperty,
    /// Interpolation mode.
    pub interpolation: InterpolationMode,
    /// Timestamps in seconds (sorted ascending).
    pub timestamps: Vec<f32>,
    /// Raw float values: 3 per keyframe for Translation/Scale, 4 for Rotation (quat xyzw).
    /// For CubicSpline: 3x keyframes (in-tangent, value, out-tangent interleaved).
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    /// Total duration in seconds.
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

// ---- AnimationClipStore ----

pub struct AnimationClipStore {
    clips: Vec<AnimationClip>,
    names: Vec<String>,
}

impl AnimationClipStore {
    pub fn new() -> Self {
        Self { clips: Vec::new(), names: Vec::new() }
    }

    pub fn add(&mut self, clip: AnimationClip) -> usize {
        let id = self.clips.len();
        self.names.push(clip.name.clone());
        self.clips.push(clip);
        id
    }

    pub fn get(&self, id: usize) -> Option<&AnimationClip> {
        self.clips.get(id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }
}

// ---- SkeletalAnimator component ----

fn default_skel_anim_speed() -> f32 { 1.0 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkeletalAnimator {
    /// Runtime skeleton ID in SkeletonStore.
    #[serde(skip)]
    pub skeleton_id: Option<usize>,
    /// Skeleton name for scene serialization (resolved at load).
    #[serde(default)]
    pub skeleton_name: Option<String>,
    /// Runtime ID of the AnimatorController in AnimatorControllerStore.
    #[serde(skip)]
    pub controller_id: Option<usize>,
    /// Name for serialization (resolved at scene load).
    #[serde(default)]
    pub controller_name: Option<String>,
    /// Runtime state of the controller (per-entity instance).
    #[serde(skip)]
    pub controller_state: Option<AnimatorControllerState>,
    /// Runtime clip IDs in AnimationClipStore.
    #[serde(skip)]
    pub clip_ids: Vec<usize>,
    /// Clip names for scene serialization.
    #[serde(default)]
    pub clip_names: Vec<String>,
    /// Active clip index within `clip_ids`. None = no animation.
    #[serde(skip)]
    pub active_clip: Option<usize>,
    /// Active clip name for serialization.
    #[serde(default)]
    pub active_clip_name: Option<String>,
    pub playing: bool,
    pub loop_animation: bool,
    #[serde(default = "default_skel_anim_speed")]
    pub speed: f32,
    /// Current playback time in seconds.
    #[serde(skip)]
    pub current_time: f32,
    /// Current local poses per bone (updated each frame by animation system).
    #[serde(skip)]
    pub current_local_poses: Vec<Transform>,
}

impl Default for SkeletalAnimator {
    fn default() -> Self {
        Self {
            skeleton_id: None,
            skeleton_name: None,
            controller_id: None,
            controller_name: None,
            controller_state: None,
            clip_ids: Vec::new(),
            clip_names: Vec::new(),
            active_clip: None,
            active_clip_name: None,
            playing: false,
            loop_animation: true,
            speed: 1.0,
            current_time: 0.0,
            current_local_poses: Vec::new(),
        }
    }
}
