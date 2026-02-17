use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ---- Parameter types ----

/// Parameter types for controlling transitions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnimatorParameter {
    Float(f32),
    Int(i32),
    Bool(bool),
    /// Auto-resets to `false` after being consumed by a transition.
    Trigger(bool),
}

impl AnimatorParameter {
    /// Extract as `f32`. Returns `0.0` for non-float variants.
    pub fn as_float(&self) -> f32 {
        match self {
            Self::Float(v) => *v,
            _ => 0.0,
        }
    }

    /// Extract as `i32`. Returns `0` for non-int variants.
    pub fn as_int(&self) -> i32 {
        match self {
            Self::Int(v) => *v,
            _ => 0,
        }
    }

    /// Extract as `bool`. Returns `false` for non-bool variants.
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            _ => false,
        }
    }

    /// Extract trigger state. Returns `false` for non-trigger variants.
    pub fn as_trigger(&self) -> bool {
        match self {
            Self::Trigger(v) => *v,
            _ => false,
        }
    }
}

// ---- Comparison / Conditions ----

/// Comparison mode for transition conditions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComparisonMode {
    /// Float/Int > threshold
    Greater,
    /// Float/Int < threshold
    Less,
    /// Int == value
    Equals,
    /// Int != value
    NotEquals,
    /// Bool == true
    IsTrue,
    /// Bool == false
    IsFalse,
    /// Trigger has been set (consumed after transition fires)
    Triggered,
}

/// A single condition on a transition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionCondition {
    /// Name of the parameter to test.
    pub parameter: String,
    pub mode: ComparisonMode,
    /// Used for `Greater` / `Less` comparisons.
    pub threshold_float: f32,
    /// Used for `Equals` / `NotEquals` comparisons.
    pub threshold_int: i32,
}

// ---- States & Transitions ----

/// An animation state in the state machine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationState {
    pub name: String,
    /// Reference to an `AnimationClip` by name.
    pub clip_name: Option<String>,
    /// Playback speed multiplier (default 1.0).
    pub speed: f32,
    /// Whether the clip loops.
    pub loop_animation: bool,
    /// Visual position in the graph editor (x, y).
    pub position: (f32, f32),
}

/// Where a transition originates from.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransitionSource {
    /// Specific state index in `AnimatorController::states`.
    State(usize),
    /// Can trigger from any state.
    AnyState,
}

/// A transition between two states.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationTransition {
    /// Source state.
    pub from_state: TransitionSource,
    /// Target state index in `AnimatorController::states`.
    pub to_state: usize,
    /// All conditions must be met (AND logic).
    pub conditions: Vec<TransitionCondition>,
    /// Crossfade duration in seconds (`0` = instant).
    pub blend_duration: f32,
    /// Wait for clip to finish before transitioning.
    pub has_exit_time: bool,
    /// Normalised time (0.0–1.0) at which exit is allowed.
    pub exit_time: f32,
}

// ---- StateChange result ----

/// Result of an `evaluate` tick.
#[derive(Clone, Debug, PartialEq)]
pub enum StateChange {
    None,
    TransitionStarted {
        from: usize,
        to: usize,
        blend_duration: f32,
    },
    BlendCompleted {
        state: usize,
    },
}

// ---- AnimatorController (asset / definition) ----

/// The main Animator Controller asset — a reusable state machine definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimatorController {
    pub name: String,
    pub states: Vec<AnimationState>,
    pub transitions: Vec<AnimationTransition>,
    pub parameters: HashMap<String, AnimatorParameter>,
    /// Index of the default/entry state in `states`.
    pub default_state: usize,
}

impl AnimatorController {
    /// Create a new, empty controller.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            states: Vec::new(),
            transitions: Vec::new(),
            parameters: HashMap::new(),
            default_state: 0,
        }
    }

    /// Add a state and return its index.
    pub fn add_state(
        &mut self,
        name: &str,
        clip_name: Option<&str>,
        speed: f32,
        loop_animation: bool,
    ) -> usize {
        let idx = self.states.len();
        self.states.push(AnimationState {
            name: name.to_string(),
            clip_name: clip_name.map(|s| s.to_string()),
            speed,
            loop_animation,
            position: (0.0, 0.0),
        });
        idx
    }

    /// Add a transition between states.
    pub fn add_transition(
        &mut self,
        from: TransitionSource,
        to: usize,
        conditions: Vec<TransitionCondition>,
        blend_duration: f32,
        has_exit_time: bool,
        exit_time: f32,
    ) {
        self.transitions.push(AnimationTransition {
            from_state: from,
            to_state: to,
            conditions,
            blend_duration,
            has_exit_time,
            exit_time,
        });
    }

    /// Set (or overwrite) a parameter.
    pub fn set_parameter(&mut self, name: &str, value: AnimatorParameter) {
        self.parameters.insert(name.to_string(), value);
    }

    /// Get a parameter by name.
    pub fn get_parameter(&self, name: &str) -> Option<&AnimatorParameter> {
        self.parameters.get(name)
    }
}

// ---- AnimatorControllerState (runtime, per-entity) ----

/// Runtime state for an active controller instance (per-entity).
#[derive(Clone, Debug, Default)]
pub struct AnimatorControllerState {
    pub current_state: usize,
    /// Previous state index (for blending).
    pub previous_state: Option<usize>,
    /// 0.0 = fully previous, 1.0 = fully current.
    pub blend_progress: f32,
    /// How long the blend takes (seconds).
    pub blend_duration: f32,
    pub is_blending: bool,
    /// Time in current state's clip (seconds).
    pub state_time: f32,
    /// Time in previous state's clip during blend (seconds).
    pub previous_state_time: f32,
    /// Runtime copy of parameters (modified per-entity).
    pub parameters: HashMap<String, AnimatorParameter>,
}

impl AnimatorControllerState {
    /// Initialise runtime state from a controller definition.
    pub fn new_from(controller: &AnimatorController) -> Self {
        Self {
            current_state: controller.default_state,
            previous_state: None,
            blend_progress: 0.0,
            blend_duration: 0.0,
            is_blending: false,
            state_time: 0.0,
            previous_state_time: 0.0,
            parameters: controller.parameters.clone(),
        }
    }

    /// Main tick function.
    ///
    /// Checks all transitions from the current state, evaluates conditions
    /// against runtime parameters. If a transition fires: sets `previous_state`,
    /// starts blend, consumes triggers. Returns what changed.
    ///
    /// `clip_duration` is the duration of the *current* state's animation clip.
    pub fn evaluate(
        &mut self,
        controller: &AnimatorController,
        dt: f32,
        clip_duration: f32,
    ) -> StateChange {
        // Advance timers
        self.state_time += dt;

        if self.is_blending {
            self.previous_state_time += dt;
            if self.blend_duration > 0.0 {
                self.blend_progress += dt / self.blend_duration;
            } else {
                self.blend_progress = 1.0;
            }
            if self.blend_progress >= 1.0 {
                self.blend_progress = 1.0;
                self.is_blending = false;
                self.previous_state = None;
                return StateChange::BlendCompleted {
                    state: self.current_state,
                };
            }
        }

        // Evaluate transitions
        let current = self.current_state;
        for transition in &controller.transitions {
            // Check source matches
            let source_matches = match &transition.from_state {
                TransitionSource::State(idx) => *idx == current,
                TransitionSource::AnyState => transition.to_state != current,
            };
            if !source_matches {
                continue;
            }

            // Check exit time gate
            if transition.has_exit_time && clip_duration > 0.0 {
                let normalised = self.state_time / clip_duration;
                if normalised < transition.exit_time {
                    continue;
                }
            }

            // Evaluate ALL conditions (AND logic)
            let mut all_met = true;
            // Track which trigger parameters to consume if we fire
            let mut triggers_to_consume: Vec<String> = Vec::new();

            for cond in &transition.conditions {
                let param = match self.parameters.get(&cond.parameter) {
                    Some(p) => p,
                    None => {
                        all_met = false;
                        break;
                    }
                };

                let met = match cond.mode {
                    ComparisonMode::Greater => param.as_float() > cond.threshold_float,
                    ComparisonMode::Less => param.as_float() < cond.threshold_float,
                    ComparisonMode::Equals => param.as_int() == cond.threshold_int,
                    ComparisonMode::NotEquals => param.as_int() != cond.threshold_int,
                    ComparisonMode::IsTrue => param.as_bool(),
                    ComparisonMode::IsFalse => !param.as_bool(),
                    ComparisonMode::Triggered => {
                        if param.as_trigger() {
                            triggers_to_consume.push(cond.parameter.clone());
                            true
                        } else {
                            false
                        }
                    }
                };

                if !met {
                    all_met = false;
                    break;
                }
            }

            if all_met {
                // Consume triggers
                for trigger_name in &triggers_to_consume {
                    if let Some(p) = self.parameters.get_mut(trigger_name) {
                        *p = AnimatorParameter::Trigger(false);
                    }
                }

                // Fire transition
                let from = self.current_state;
                let to = transition.to_state;
                let blend = transition.blend_duration;

                self.previous_state = Some(from);
                self.previous_state_time = self.state_time;
                self.current_state = to;
                self.state_time = 0.0;

                if blend > 0.0 {
                    self.is_blending = true;
                    self.blend_progress = 0.0;
                    self.blend_duration = blend;
                } else {
                    self.is_blending = false;
                    self.blend_progress = 1.0;
                    self.blend_duration = 0.0;
                    self.previous_state = None;
                }

                return StateChange::TransitionStarted {
                    from,
                    to,
                    blend_duration: blend,
                };
            }
        }

        StateChange::None
    }

    // ---- Parameter setters ----

    pub fn set_float(&mut self, name: &str, value: f32) {
        self.parameters
            .insert(name.to_string(), AnimatorParameter::Float(value));
    }

    pub fn set_int(&mut self, name: &str, value: i32) {
        self.parameters
            .insert(name.to_string(), AnimatorParameter::Int(value));
    }

    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.parameters
            .insert(name.to_string(), AnimatorParameter::Bool(value));
    }

    /// Sets a trigger to `true`. It will be consumed (reset to `false`)
    /// when a transition using it fires.
    pub fn set_trigger(&mut self, name: &str) {
        self.parameters
            .insert(name.to_string(), AnimatorParameter::Trigger(true));
    }

    // ---- Clip name helpers ----

    /// Get the clip name of the current state.
    pub fn get_current_clip_name<'a>(
        &self,
        controller: &'a AnimatorController,
    ) -> Option<&'a str> {
        controller
            .states
            .get(self.current_state)
            .and_then(|s| s.clip_name.as_deref())
    }

    /// Get the clip name of the previous state (for blending).
    pub fn get_previous_clip_name<'a>(
        &self,
        controller: &'a AnimatorController,
    ) -> Option<&'a str> {
        self.previous_state
            .and_then(|idx| controller.states.get(idx))
            .and_then(|s| s.clip_name.as_deref())
    }
}

// ---- AnimatorControllerStore (shared asset store) ----

pub struct AnimatorControllerStore {
    controllers: Vec<AnimatorController>,
    names: Vec<String>,
}

impl AnimatorControllerStore {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            names: Vec::new(),
        }
    }

    pub fn add(&mut self, controller: AnimatorController) -> usize {
        let id = self.controllers.len();
        self.names.push(controller.name.clone());
        self.controllers.push(controller);
        id
    }

    pub fn get(&self, id: usize) -> Option<&AnimatorController> {
        self.controllers.get(id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.controllers.len()
    }
}
