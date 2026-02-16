# ClawdEngine Scripting API Reference

## 1. Overview

Scripts in ClawdEngine are Rust structs that implement the `GameScript` trait. They are attached to entities and executed during play mode. The engine calls `start()` once when play mode begins, then `update()` every frame.

### Lifecycle

1. Define a struct that implements `GameScript`.
2. Register it in the script registry (in `default_scene.rs` or your scene setup).
3. Attach it to an entity as a `Box<dyn GameScript>`.
4. When play mode starts, the engine calls `start()` on every script.
5. Each frame, the engine calls `update()` with a `ScriptContext` and `dt` (delta time in seconds).
6. Entity destruction is deferred: calls to `ctx.destroy_entity()` are applied after all scripts have run.

### Registration pattern

```rust
use crate::scripting::{GameScript, ScriptContext};
use crate::core::EntityId;

// In your scene setup function, return scripts paired with their entities:
fn setup_scene(world: &mut World) -> Vec<(EntityId, Box<dyn GameScript>)> {
    let player = world.spawn_entity();
    world.set_name(player, "Player");
    world.set_transform(player, Transform::default());

    vec![
        (player, Box::new(MyPlayerScript::new()) as Box<dyn GameScript>),
    ]
}
```

---

## 2. GameScript Trait

```rust
pub trait GameScript {
    /// Display name shown in the editor inspector.
    fn name(&self) -> &str { "Script" }

    /// Called once when play mode starts.
    fn start(&mut self, ctx: &mut ScriptContext) {}

    /// Called every frame during play mode.
    /// `dt` is the frame delta time in seconds.
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {}

    /// Draw custom inspector fields in the editor (edit mode).
    fn inspector_ui(&mut self, ui: &mut egui::Ui) {}

    /// Draw HUD overlay in the Game View (play mode only).
    fn game_ui(&mut self, ui: &mut egui::Ui) {}
}
```

### Method details

| Method | When called | Purpose |
|--------|-------------|---------|
| `name()` | Editor display | Returns the script name for the inspector panel |
| `start()` | Once at play start | Initialize state, cache entity references, set initial velocities |
| `update()` | Every frame | Main game logic -- movement, input handling, collision response |
| `inspector_ui()` | Editor (always) | Expose tunable parameters (speed, health, etc.) via egui widgets |
| `game_ui()` | Play mode | Draw HUD elements (health bars, crosshairs, score) on the game viewport |

---

## 3. ScriptContext

`ScriptContext` is the gateway to the engine. It provides access to the entity's own components, other entities, input, physics, and more.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `entity` | `EntityId` | The entity this script is attached to |
| `world` | `&mut World` | Full mutable access to the World (advanced use) |
| `time` | `f32` | Total elapsed time since play mode started (seconds) |
| `dt` | `f32` | Delta time for the current frame (seconds) |

### Self-entity component access

These convenience methods operate on the script's own entity (`ctx.entity`).

```rust
// Transform
ctx.get_transform() -> Option<&Transform>
ctx.get_transform_mut() -> Option<&mut Transform>

// RigidBody
ctx.get_rigid_body() -> Option<&RigidBody>
ctx.get_rigid_body_mut() -> Option<&mut RigidBody>

// Material
ctx.get_material() -> Option<&Material>
ctx.get_material_mut() -> Option<&mut Material>

// AudioSource
ctx.get_audio_source() -> Option<&AudioSource>
ctx.get_audio_source_mut() -> Option<&mut AudioSource>

// Audio shortcuts
ctx.play_audio()   // Sets is_playing = true
ctx.stop_audio()   // Sets is_playing = false
```

### Cross-entity access

```rust
// Find entities
ctx.find_by_name("Player") -> Option<EntityId>
ctx.find_all_by_name("Enemy") -> Vec<EntityId>  // substring match
ctx.is_alive(entity_id) -> bool

// Read/write other entities
ctx.get_entity_transform(target) -> Option<&Transform>
ctx.get_entity_transform_mut(target) -> Option<&mut Transform>
ctx.get_entity_material_mut(target) -> Option<&mut Material>
ctx.get_entity_rigid_body_mut(target) -> Option<&mut RigidBody>
ctx.get_entity_name(target) -> Option<&str>

// Create/destroy
ctx.spawn_entity() -> EntityId
ctx.destroy_entity(target)  // Deferred: applied after all scripts run
```

### Tags

Tags are runtime-only string labels attached to entities. They are not serialized in scene files.

```rust
ctx.add_tag(entity, "collectible")
ctx.has_tag(entity, "collectible") -> bool
ctx.find_by_tag("enemy") -> Vec<EntityId>
ctx.remove_tag(entity, "collectible")
```

### Raycasting

```rust
// Cast a ray from any origin in any direction
ctx.raycast(origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit>
ctx.raycast_all(origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RayHit>

// Cast a ray forward from this entity's position and orientation
ctx.raycast_forward(max_distance: f32) -> Option<RayHit>
```

#### RayHit struct

```rust
pub struct RayHit {
    pub entity: EntityId,   // The entity that was hit
    pub point: Vec3,        // World-space hit position
    pub normal: Vec3,       // Surface normal at hit point
    pub distance: f32,      // Distance from ray origin to hit
}
```

### Collision queries

```rust
ctx.collisions() -> impl Iterator<Item = &CollisionEvent>
```

Returns only collision events involving the current entity. See Section 6 for details.

### Input

```rust
// Keyboard (physical key codes, AZERTY-compatible)
ctx.is_key_held(KeyCode) -> bool       // True while key is down
ctx.is_key_pressed(KeyCode) -> bool    // True only on the frame the key was pressed
ctx.is_key_released(KeyCode) -> bool   // True only on the frame the key was released

// Mouse
ctx.is_mouse_held(MouseButton) -> bool
ctx.is_mouse_pressed(MouseButton) -> bool
ctx.mouse_position() -> [f32; 2]       // Cursor position in window pixels
ctx.mouse_delta() -> [f32; 2]          // Cursor movement since last frame
ctx.scroll_delta() -> [f32; 2]         // Scroll wheel delta [horizontal, vertical]
```

---

## 4. Components Reference

All components are defined in `src/core/components.rs`. They are accessed through the World API via `get_*` / `set_*` / `remove_*` methods.

### Transform

Position, rotation, and scale of an entity in local space.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `position` | `Vec3` | `(0, 0, 0)` | Local position |
| `rotation` | `Quat` | `IDENTITY` | Local rotation (quaternion) |
| `scale` | `Vec3` | `(1, 1, 1)` | Local scale |

### MeshRenderer

Links an entity to a mesh for 3D rendering.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mesh_id` | `Option<usize>` | `None` | Index into the MeshStore |
| `visible` | `bool` | `true` | Whether to render this mesh |

### Material

Surface appearance properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `albedo` | `Vec3` | `(0.8, 0.8, 0.8)` | Base color (RGB, 0-1) |
| `roughness` | `f32` | `0.5` | Surface roughness (0 = mirror, 1 = matte) |
| `metallic` | `f32` | `0.0` | Metalness (0 = dielectric, 1 = metal) |
| `emission` | `Vec3` | `(0, 0, 0)` | Emissive color (added after lighting) |
| `texture_path` | `Option<String>` | `None` | Path to albedo texture file |
| `texture_id` | `Option<usize>` | `None` | Runtime texture handle (not serialized) |
| `normal_map_path` | `Option<String>` | `None` | Path to normal map file |
| `normal_map_id` | `Option<usize>` | `None` | Runtime normal map handle (not serialized) |

### Light

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | `LightKind` | `Directional` | `Directional`, `Point`, or `Spot` |
| `color` | `Vec3` | `(1.0, 0.95, 0.85)` | Light color (RGB) |
| `intensity` | `f32` | `1.5` | Brightness multiplier |
| `range` | `f32` | `10.0` | Attenuation range (Point/Spot only) |
| `inner_angle` | `f32` | `30 deg` | Inner cone angle in radians (Spot only) |
| `outer_angle` | `f32` | `45 deg` | Outer cone angle in radians (Spot only) |

### RigidBody

Physics body with velocity and gravity.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mass` | `f32` | `1.0` | Mass in arbitrary units |
| `gravity_enabled` | `bool` | `true` | Whether gravity applies |
| `velocity` | `Vec3` | `(0, 0, 0)` | Linear velocity (units/second) |
| `angular_velocity` | `Vec3` | `(0, 0, 0)` | Angular velocity (rad/second) |

### Collider

Defines a collision shape for physics and raycasting.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `shape` | `ColliderShape` | `Box` | `Box` or `Sphere` |
| `center` | `Vec3` | `(0, 0, 0)` | Offset from entity origin (local space) |
| `half_extents` | `Vec3` | `(0.5, 0.5, 0.5)` | Half-size for Box shape |
| `radius` | `f32` | `0.5` | Radius for Sphere shape |
| `restitution` | `f32` | `0.3` | Bounciness (0 = no bounce, 1 = perfect bounce) |
| `friction` | `f32` | `0.5` | Surface friction |
| `is_trigger` | `bool` | `false` | If true, detects overlaps but does not resolve |

### CameraComponent

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fov_y` | `f32` | `60 deg` | Vertical field of view in radians |
| `near` | `f32` | `0.1` | Near clip plane |
| `far` | `f32` | `100.0` | Far clip plane |
| `is_main` | `bool` | `true` | Whether this is the active game camera |

### AudioSource

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `audio_path` | `Option<String>` | `None` | Path to audio file (WAV) |
| `volume` | `f32` | `1.0` | Playback volume (0-1) |
| `pitch` | `f32` | `1.0` | Playback speed multiplier |
| `loop_audio` | `bool` | `false` | Whether to loop playback |
| `play_on_start` | `bool` | `false` | Auto-play when play mode begins |
| `spatial` | `bool` | `false` | Enable distance-based attenuation |
| `max_distance` | `f32` | `20.0` | Maximum audible distance (spatial only) |
| `is_playing` | `bool` | `false` | Runtime playback state (not serialized) |

### AudioListener

Required on an entity (typically the camera) for audio to be heard.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `active` | `bool` | `true` | Whether this listener is active |
| `volume` | `f32` | `1.0` | Master volume multiplier |

### UiElement

On-screen UI element, must be a child of a Canvas entity.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | `UiElementKind` | `Text` | `Text` or `Panel` |
| `text` | `String` | `"Hello World"` | Display text (Text kind) |
| `font_size` | `f32` | `16.0` | Font size in pixels |
| `color` | `Vec3` | `(1, 1, 1)` | Text/panel color (RGB) |
| `alpha` | `f32` | `1.0` | Opacity (0 = transparent, 1 = opaque) |
| `anchor` | `UiAnchor` | `TopLeft` | Screen anchor position |
| `offset` | `[f32; 2]` | `[16, 16]` | Pixel offset from anchor |
| `size` | `[f32; 2]` | `[200, 40]` | Size in pixels (Panel kind) |
| `visible` | `bool` | `true` | Whether to render |

#### UiAnchor values

`TopLeft`, `TopCenter`, `TopRight`, `CenterLeft`, `Center`, `CenterRight`, `BottomLeft`, `BottomCenter`, `BottomRight`

### Canvas

Container for UiElement children.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `f32` | `1920.0` | Reference resolution width |
| `height` | `f32` | `1080.0` | Reference resolution height |
| `visible` | `bool` | `true` | Whether to render child UI elements |

---

## 5. Input API

### Key codes

Key codes are **physical** (position-based), not layout-dependent. On AZERTY keyboards, the physical WASD positions map to ZQSD on the keycaps, but in code you always use `KeyCode::KeyW`, `KeyCode::KeyA`, etc.

Commonly used key codes:

| KeyCode | Physical key |
|---------|-------------|
| `KeyCode::KeyW` | Top-left of home row (W on QWERTY, Z on AZERTY) |
| `KeyCode::KeyA` | Left of home row |
| `KeyCode::KeyS` | Home row center-left |
| `KeyCode::KeyD` | Home row center-right |
| `KeyCode::Space` | Spacebar |
| `KeyCode::ShiftLeft` | Left Shift |
| `KeyCode::ShiftRight` | Right Shift |
| `KeyCode::ControlLeft` | Left Control |
| `KeyCode::ArrowUp` | Up arrow |
| `KeyCode::ArrowDown` | Down arrow |
| `KeyCode::ArrowLeft` | Left arrow |
| `KeyCode::ArrowRight` | Right arrow |
| `KeyCode::Escape` | Escape |
| `KeyCode::Enter` | Enter/Return |
| `KeyCode::KeyE` | E key position |
| `KeyCode::KeyQ` | Q key position |
| `KeyCode::KeyF` | F key position |
| `KeyCode::Digit1` .. `KeyCode::Digit9` | Number row |

Full list: see `winit::keyboard::KeyCode` documentation.

### Mouse buttons

```rust
MouseButton::Left
MouseButton::Right
MouseButton::Middle
```

### Input method summary

| Method | Returns | Description |
|--------|---------|-------------|
| `is_key_held(key)` | `bool` | True while the key is held down |
| `is_key_pressed(key)` | `bool` | True only on the first frame the key goes down |
| `is_key_released(key)` | `bool` | True only on the frame the key is released |
| `is_mouse_held(btn)` | `bool` | True while the mouse button is held |
| `is_mouse_pressed(btn)` | `bool` | True on first frame of mouse button press |
| `mouse_position()` | `[f32; 2]` | Cursor position in window-space pixels |
| `mouse_delta()` | `[f32; 2]` | Cursor movement since the previous frame |
| `scroll_delta()` | `[f32; 2]` | Scroll wheel delta `[horizontal, vertical]` |

---

## 6. Collision Events

Collisions are detected automatically between entities that have both a `Transform` and a `Collider`. Events are delivered to scripts via `ctx.collisions()`.

### CollisionEvent struct

```rust
pub struct CollisionEvent {
    pub entity: EntityId,           // One of the two colliding entities
    pub other: EntityId,            // The other entity
    pub kind: CollisionEventKind,   // Enter, Stay, or Exit
    pub normal: Vec3,               // Collision normal
}
```

### CollisionEventKind

| Variant | Meaning |
|---------|---------|
| `Enter` | First frame of contact (just started touching) |
| `Stay` | Continuing contact from previous frame |
| `Exit` | First frame after contact ended (just separated) |

### Trigger colliders

When `Collider.is_trigger = true`, the collider detects overlaps but does not push entities apart. Useful for pickups, checkpoints, and area detection.

### Usage

```rust
fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
    for collision in ctx.collisions() {
        match collision.kind {
            CollisionEventKind::Enter => {
                // Just entered contact with collision.other
            }
            CollisionEventKind::Stay => {
                // Still in contact
            }
            CollisionEventKind::Exit => {
                // Just left contact
            }
        }
    }
}
```

Note: `ctx.collisions()` returns only events where `event.entity == ctx.entity`. Each collision pair generates events for both entities involved.

---

## 7. Math Utilities

The scripting module re-exports several math helpers from `src/scripting/math_utils.rs`.

```rust
use crate::scripting::{lerp, lerp_vec3, distance, distance_squared, look_at_rotation, move_toward, GameRng};
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `lerp` | `(a: f32, b: f32, t: f32) -> f32` | Linear interpolation between two floats |
| `lerp_vec3` | `(a: Vec3, b: Vec3, t: f32) -> Vec3` | Linear interpolation between two vectors |
| `distance` | `(a: Vec3, b: Vec3) -> f32` | Euclidean distance between two points |
| `distance_squared` | `(a: Vec3, b: Vec3) -> f32` | Squared distance (avoids sqrt) |
| `look_at_rotation` | `(from: Vec3, target: Vec3) -> Quat` | Quaternion that looks from `from` toward `target` (Y-up) |
| `move_toward` | `(current: f32, target: f32, max_step: f32) -> f32` | Move a value toward target by at most `max_step` |
| `GameRng::new(seed)` | `-> GameRng` | Xorshift32 pseudo-random generator |
| `rng.random_f32()` | `-> f32` | Random float in `[0.0, 1.0)` |
| `rng.random_range(min, max)` | `-> f32` | Random float in `[min, max)` |

---

## 8. Examples

### 8.1 Player movement controller (WASD + jump)

A simple third-person movement script with WASD horizontal movement, Space to jump, and ground detection.

```rust
use crate::scripting::{GameScript, ScriptContext, KeyCode};
use glam::Vec3;

pub struct PlayerController {
    speed: f32,
    jump_force: f32,
    grounded: bool,
}

impl PlayerController {
    pub fn new() -> Self {
        Self {
            speed: 6.0,
            jump_force: 8.0,
            grounded: false,
        }
    }
}

impl GameScript for PlayerController {
    fn name(&self) -> &str { "Player Controller" }

    fn start(&mut self, ctx: &mut ScriptContext) {
        // Ensure gravity is enabled
        if let Some(rb) = ctx.get_rigid_body_mut() {
            rb.gravity_enabled = true;
        }
    }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        // Ground check: treat Y <= 0.05 as grounded
        self.grounded = ctx.get_transform()
            .is_some_and(|t| t.position.y <= 0.05);

        // Horizontal movement (WASD)
        let mut dir = Vec3::ZERO;
        if ctx.is_key_held(KeyCode::KeyW) { dir.z -= 1.0; }
        if ctx.is_key_held(KeyCode::KeyS) { dir.z += 1.0; }
        if ctx.is_key_held(KeyCode::KeyA) { dir.x -= 1.0; }
        if ctx.is_key_held(KeyCode::KeyD) { dir.x += 1.0; }

        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }

        if let Some(t) = ctx.get_transform_mut() {
            t.position += dir * self.speed * dt;
        }

        // Jump (Space, only when grounded)
        if self.grounded && ctx.is_key_pressed(KeyCode::Space) {
            if let Some(rb) = ctx.get_rigid_body_mut() {
                rb.velocity.y = self.jump_force;
            }
        }
    }
}
```

### 8.2 Collectible pickup (trigger collision, then destroy)

A collectible item that destroys itself and increments a score when a player enters its trigger collider.

```rust
use crate::scripting::{GameScript, ScriptContext};
use crate::physics::collision::CollisionEventKind;

pub struct Collectible {
    points: u32,
    collected: bool,
}

impl Collectible {
    pub fn new(points: u32) -> Self {
        Self { points, collected: false }
    }
}

impl GameScript for Collectible {
    fn name(&self) -> &str { "Collectible" }

    fn update(&mut self, ctx: &mut ScriptContext, _dt: f32) {
        if self.collected {
            return;
        }

        for collision in ctx.collisions() {
            if collision.kind == CollisionEventKind::Enter {
                // Check if the other entity is the player
                let is_player = ctx.get_entity_name(collision.other)
                    .is_some_and(|name| name == "Player");

                if is_player {
                    self.collected = true;
                    ctx.destroy_entity(ctx.entity); // Remove this collectible
                    // Optionally play a pickup sound:
                    // ctx.play_audio();
                    break;
                }
            }
        }
    }
}
```

Setup requirements: the collectible entity must have a `Collider` with `is_trigger = true` and a `Transform`.

### 8.3 Rotating object

A configurable script that rotates an entity around one or more axes.

```rust
use crate::scripting::{GameScript, ScriptContext};
use glam::Quat;

pub struct Rotator {
    speed_x: f32,
    speed_y: f32,
    speed_z: f32,
    angle_x: f32,
    angle_y: f32,
    angle_z: f32,
}

impl Rotator {
    pub fn new(speed_y: f32) -> Self {
        Self {
            speed_x: 0.0,
            speed_y,
            speed_z: 0.0,
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
        }
    }
}

impl GameScript for Rotator {
    fn name(&self) -> &str { "Rotator" }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        self.angle_x += self.speed_x * dt;
        self.angle_y += self.speed_y * dt;
        self.angle_z += self.speed_z * dt;

        if let Some(t) = ctx.get_transform_mut() {
            t.rotation = Quat::from_euler(
                glam::EulerRot::XYZ,
                self.angle_x,
                self.angle_y,
                self.angle_z,
            );
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Speed X");
            ui.add(egui::DragValue::new(&mut self.speed_x).speed(0.05));
        });
        ui.horizontal(|ui| {
            ui.label("Speed Y");
            ui.add(egui::DragValue::new(&mut self.speed_y).speed(0.05));
        });
        ui.horizontal(|ui| {
            ui.label("Speed Z");
            ui.add(egui::DragValue::new(&mut self.speed_z).speed(0.05));
        });
    }
}
```

### 8.4 Audio trigger (play sound on collision)

Plays a sound effect when another entity enters this entity's collider, with a cooldown to prevent re-triggering.

```rust
use crate::scripting::{GameScript, ScriptContext};
use crate::physics::collision::CollisionEventKind;

pub struct AudioTrigger {
    cooldown: f32,
    timer: f32,
}

impl AudioTrigger {
    pub fn new(cooldown: f32) -> Self {
        Self { cooldown, timer: 0.0 }
    }
}

impl GameScript for AudioTrigger {
    fn name(&self) -> &str { "Audio Trigger" }

    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        // Decrease cooldown timer
        if self.timer > 0.0 {
            self.timer -= dt;
        }

        // Check for new collisions
        for collision in ctx.collisions() {
            if collision.kind == CollisionEventKind::Enter && self.timer <= 0.0 {
                ctx.play_audio();
                self.timer = self.cooldown;
                break;
            }
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Cooldown (s)");
            ui.add(egui::DragValue::new(&mut self.cooldown)
                .speed(0.1)
                .range(0.0..=10.0));
        });
    }
}
```

Setup requirements: the entity must have a `Transform`, a `Collider`, and an `AudioSource` with a valid `audio_path`.

---

## Appendix: World API (direct access)

For advanced use, `ctx.world` provides full mutable access to the World. Component access follows a consistent pattern for all types:

```rust
// Set a component on an entity
ctx.world.set_transform(entity, Transform { ... });
ctx.world.set_material(entity, Material { ... });
ctx.world.set_collider(entity, Collider { ... });
// ... same pattern for all component types

// Get a component (immutable)
ctx.world.get_transform(entity) -> Option<&Transform>

// Get a component (mutable)
ctx.world.get_transform_mut(entity) -> Option<&mut Transform>

// Remove a component
ctx.world.remove_transform(entity);

// Entity management
ctx.world.spawn_entity() -> EntityId
ctx.world.destroy_entity(entity)
ctx.world.is_alive(entity) -> bool
ctx.world.entity_count() -> usize
ctx.world.iter_entities() -> impl Iterator<Item = EntityId>

// Naming
ctx.world.set_name(entity, "MyEntity")
ctx.world.get_name(entity) -> Option<&str>

// Hierarchy
ctx.world.set_parent(child, parent)
ctx.world.remove_parent(child)
ctx.world.get_parent(entity) -> Option<EntityId>
ctx.world.get_children(entity) -> &[EntityId]
ctx.world.get_world_transform(entity) -> Option<Transform>  // Resolved through parent chain

// Custom components (TypeMap, runtime-only)
ctx.world.add_custom::<MyData>(entity, data)
ctx.world.get_custom::<MyData>(entity) -> Option<&MyData>
ctx.world.get_custom_mut::<MyData>(entity) -> Option<&mut MyData>
ctx.world.remove_custom::<MyData>(entity)
```

Available `set_*` / `get_*` / `get_*_mut` / `remove_*` methods exist for: `transform`, `mesh_renderer`, `material`, `light`, `rigid_body`, `collider`, `camera`, `audio_source`, `audio_listener`, `ui_element`, `canvas`.
