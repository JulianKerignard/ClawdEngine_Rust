# ClawdEngine

A 3D game engine built from scratch in Rust, created for the Claude Code hackathon. ClawdEngine provides a complete editor experience with real-time 3D rendering, physics, scripting, audio, and one-click macOS game export -- all in ~48,000 lines of Rust and 315 lines of WGSL shaders.

---

## Screenshots

<!-- Add screenshots here -->
*Coming soon*

---

## Features

### Rendering
- GPU-accelerated rendering via **wgpu** (WebGPU/Metal/Vulkan/DX12)
- Forward rendering with PBR-inspired materials (albedo, roughness, metallic, emission)
- Normal mapping with tangent-space TBN computation
- Shadow mapping with depth-only pass and 4-tap PCF soft shadows
- Procedural gradient skybox
- Directional, point, and spot lights with UE4-style distance attenuation
- Configurable shadow map resolution (up to 4096x4096)
- Render-to-texture viewport (scene view and game view run independently)
- Line-based grid, gizmos, and selection outlines
- 6 built-in procedural meshes: Cube, Sphere, Plane, Cylinder, Capsule, Cone
- OBJ and glTF/GLB model import with texture loading

### Editor
- Full dock-based panel layout powered by **egui** and **egui_dock**
- **Hierarchy** panel with search, drag-and-drop reparenting, and entity tree
- **Viewport** panel with orbit/pan/zoom camera and translate/rotate/scale gizmos
- **Inspector** panel for editing transforms, materials, physics, audio, scripts, and UI components
- **Assets** browser with folder navigation, file creation (scenes, scripts, folders), and drag-to-load
- **Console** panel with log levels (Info, Warn, Error, Debug), filtering, and auto-scroll
- **Game View** panel showing the game camera perspective in real time
- **Settings** panel for rendering, shadows, lighting, camera, grid, physics, and build configuration
- Multi-selection with Shift+Click
- Entity spawning menu (meshes, lights, cameras, audio sources, UI elements)
- Add Component workflow (Material, RigidBody, Collider, Camera, AudioSource, AudioListener, Canvas, UiElement)
- Undo/Redo with up to 32 history entries
- Scene save/load in RON format with preview thumbnails
- Dark theme with custom styling

### Physics
- Gravity simulation with configurable strength and ground plane
- AABB and sphere collider shapes
- Box-Box, Sphere-Sphere, and Sphere-Box collision detection
- Impulse-based collision resolution with restitution and friction
- Position correction to prevent interpenetration
- Trigger colliders (collision events without physical response)
- Rigid body dynamics with mass, velocity, and angular velocity
- Ground plane collision with friction damping
- Raycasting API (single hit, all hits, forward ray)
- Collision event system (Enter, Stay, Exit)

### Scripting
- Trait-based scripting system (`GameScript` trait)
- `start()` and `update(dt)` lifecycle hooks
- Full `ScriptContext` API providing access to:
  - Entity transforms, materials, rigid bodies, audio sources
  - Keyboard and mouse input (held, pressed, released, delta)
  - Collision events and raycasting
  - Cross-entity queries (`find_by_name`, `find_by_tag`)
  - Entity spawn and destroy
  - Runtime tags system
- Custom inspector UI per script (`inspector_ui`)
- Game HUD overlay per script (`game_ui`)
- 7 built-in demo scripts: Rotate & Pulse, Gravity Bounce, Color Cycle, Oscillate, Audio Demo, Player HUD, FPS Controller
- Math utilities: lerp, distance, look_at, move_toward, seeded RNG

### Audio
- Spatial audio powered by **rodio**
- AudioSource component (volume, pitch, loop, play-on-start, spatial, max distance)
- AudioListener component (Unity-style: audio only plays when a listener exists)
- Distance-based attenuation for spatial sources
- Per-entity sink management with automatic cleanup

### Scene System
- Serialization in RON format (`.ron` files)
- Scene thumbnails (PNG previews)
- Create, save, load, and switch scenes from the editor
- Entity hierarchy preservation across save/load
- Script references serialized by name with registry-based re-instantiation

### Play Mode
- In-editor play mode with world snapshot/restore
- Fullscreen game mode (F5 to enter, ESC to exit)
- Game View panel for side-by-side editing and testing
- Scripts, physics, and audio activate only during play

### Game Export
- One-click macOS `.app` bundle export (Build Game)
- Standalone player mode with `game.ron` manifest
- Assets bundled into `Resources/` directory
- CLI support: `--player <scene.ron> --name "Game Name"`
- Auto-detection of bundled game mode on launch

### Project Management
- Project Hub with create, open, rename, and delete operations
- Blank and demo project templates
- Recent projects list with thumbnails, sorted by date or name
- Per-project settings persisted in `settings.ron`
- Project manifest (`project.ron`) with metadata

### Input
- AZERTY and QWERTY keyboard layout support
- Dual input tracking: physical keys (KeyCode) for tools, logical keys for shortcuts
- Mouse: left (select/gizmo), right (orbit + fly), middle (pan), scroll (zoom)
- Touch and pixel-delta scroll normalization

---

## Getting Started

### Prerequisites

- **Rust** (stable, 2021 edition) -- install from [rustup.rs](https://rustup.rs)
- macOS, Windows, or Linux (primary development on macOS)

### Build

```bash
git clone https://github.com/your-username/clawdengine.git
cd clawdengine
cargo build
```

### Run

```bash
cargo run
```

The engine opens to the **Project Hub**. Create a new project (blank or demo scene) to enter the editor.

### Release Build

```bash
cargo build --release
```

The release profile enables `opt-level = 3`, thin LTO, symbol stripping, and single codegen unit for maximum performance.

---

## Architecture

ClawdEngine is organized into 7 modules plus a thin `main.rs` application shell:

```
src/
  main.rs              # Application entry, winit event loop, frame orchestration
  core/                # ECS-like world: entities, components (Transform, Material,
                       #   Light, RigidBody, Collider, Camera, Audio, UI), world snapshots
  renderer/            # wgpu GPU context, mesh pipeline (5 bind groups), shadow map,
                       #   skybox, line pipeline, camera, viewport textures, texture store
  editor/              # egui UI: dock layout, 7 panel types, project hub, gizmo
                       #   interaction, picking, shortcuts, undo/redo, theme, asset loader
  physics/             # Gravity, collision detection (AABB/sphere), impulse resolution,
                       #   position correction, ground plane, raycasting, collision events
  scripting/           # GameScript trait, ScriptContext API, demo scripts, FPS controller,
                       #   math utilities
  input/               # Keyboard (physical + logical), mouse, scroll state tracking
  assets/              # Scene serialization (RON), project manifest, settings, OBJ loader,
                       #   glTF loader, path resolution (.app bundle aware)
  audio/               # rodio integration, spatial audio, AudioListener pattern, sink management
```

### Rendering Pipeline

The renderer uses a dual-pass architecture:

1. **Shadow pass** -- Depth-only render from the directional light's perspective into a shadow map
2. **Scene pass** -- Forward render all meshes into an off-screen texture with 5 bind groups:
   - Group 0: Camera (view-projection matrix, eye position)
   - Group 1: Model (model matrix, per-object color)
   - Group 2: Material (albedo texture, sampler, normal map, material uniforms)
   - Group 3: Lights (up to N lights with position, color, direction, spot params)
   - Group 4: Shadow (shadow map texture, comparison sampler, light VP matrix)
3. **Skybox pass** -- Full-screen gradient behind the scene
4. **Lines pass** -- Grid, gizmo axes, and selection outlines
5. **egui pass** -- Editor UI composited on the swap chain, with the scene texture displayed in the Viewport panel

### Shaders

All shaders are written in WGSL:

| Shader | Purpose |
|---|---|
| `mesh.wgsl` | PBR-ish forward lighting, normal mapping, shadow sampling (PCF) |
| `shadow.wgsl` | Depth-only pass for shadow map generation |
| `skybox.wgsl` | Procedural gradient skybox |
| `lines.wgsl` | Colored line rendering for grid and gizmos |

---

## Controls

### Viewport Camera

| Input | Action |
|---|---|
| Right-click + drag | Orbit camera |
| Right-click + WASD | Fly mode (forward/back/strafe) |
| Right-click + Q/E | Fly down/up |
| Middle-click + drag | Pan camera |
| Scroll wheel | Zoom in/out |
| F | Focus on selected entity |

### Editor Shortcuts

| Shortcut | Action |
|---|---|
| Q | Select tool |
| W | Move tool |
| E | Rotate tool |
| R | Scale tool |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+S | Save scene |
| Cmd+O | Load scene |
| Cmd+D | Duplicate selection |
| Shift+A | Toggle quick-add menu |
| Delete / Backspace | Delete selected entities |
| Escape | Deselect all / exit fullscreen |
| F5 | Enter fullscreen game mode |

All Cmd shortcuts also work with Ctrl. Keyboard layout (AZERTY/QWERTY) is handled automatically: tool keys use physical position, Cmd shortcuts use logical characters.

---

## Creating a Game

1. **Create a project** -- Launch the engine and click "New Project" in the Project Hub. Choose a blank or demo template.

2. **Build your scene** -- Use the Hierarchy panel's "+" button or Shift+A to add entities (cubes, spheres, lights, cameras). Position them with the Move/Rotate/Scale gizmos.

3. **Add materials** -- Select an entity and edit its Material in the Inspector. Set albedo color, roughness, metallic, emission. Assign textures and normal maps from the Assets panel.

4. **Add physics** -- Attach RigidBody and Collider components via "Add Component" in the Inspector. Configure mass, restitution, friction, and trigger mode.

5. **Write scripts** -- Create a `.rs` script in the Assets panel or attach one of the built-in scripts (Rotate & Pulse, FPS Controller, etc.). Scripts have access to transforms, input, collisions, raycasting, audio, and cross-entity queries.

6. **Test in play mode** -- Click the Play button or press F5 for fullscreen. The world state is snapshot before play and restored on stop.

7. **Export** -- Open the Settings panel, configure Build settings (app name, bundle ID, version), and click "Build Game" to generate a standalone macOS `.app` bundle.

---

## Tech Stack

| Dependency | Version | Purpose |
|---|---|---|
| [wgpu](https://wgpu.rs) | 27.0.1 | GPU rendering (WebGPU/Metal/Vulkan/DX12) |
| [egui](https://github.com/emilk/egui) | 0.33.3 | Immediate-mode editor UI |
| [egui-wgpu](https://github.com/emilk/egui) | 0.33.3 | egui rendering backend for wgpu |
| [egui-winit](https://github.com/emilk/egui) | 0.33.3 | egui input integration with winit |
| [egui_dock](https://github.com/Adanos020/egui_dock) | 0.18.0 | Dockable panel layout |
| [winit](https://github.com/rust-windowing/winit) | 0.30.12 | Window creation and event loop |
| [glam](https://github.com/bitshifter/glam-rs) | 0.31.0 | Linear algebra (Vec3, Mat4, Quat) |
| [rodio](https://github.com/RustAudio/rodio) | 0.21 | Audio playback and spatial audio |
| [tobj](https://github.com/Twinklebear/tobj) | 4.0.3 | OBJ model loading |
| [gltf](https://github.com/gltf-rs/gltf) | 1.4 | glTF/GLB model loading |
| [image](https://github.com/image-rs/image) | 0.25.5 | Texture loading (PNG, JPG, etc.) |
| [serde](https://serde.rs) | 1.0.217 | Serialization framework |
| [ron](https://github.com/ron-rs/ron) | 0.8.1 | Rusty Object Notation (scene/settings format) |
| [bytemuck](https://github.com/Lokathor/bytemuck) | 1.25.0 | Safe byte casting for GPU buffers |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.15 | Native file dialogs |
| [chrono](https://github.com/chronotope/chrono) | 0.4 | Date/time for project metadata |
| [anyhow](https://github.com/dtolnay/anyhow) | 1.0.101 | Error handling |
| [log](https://github.com/rust-lang/log) + [env_logger](https://github.com/rust-cli/env_logger) | 0.4 / 0.11 | Logging framework |

---

## Project Structure

```
clawdengine/
  Cargo.toml             # Dependencies and build profiles
  src/                   # Rust source (~48k lines)
    main.rs              # Entry point and application loop
    core/                # Entity system, components, world
    renderer/            # GPU context, pipelines, camera, viewport
    editor/              # UI panels, project hub, gizmos, shortcuts
    physics/             # Collision, raycasting, rigid body dynamics
    scripting/           # Script trait, context API, demo scripts
    input/               # Keyboard and mouse state
    assets/              # Serialization, project management, loaders
    audio/               # Audio system with spatial support
  shaders/               # WGSL shader sources
    mesh.wgsl            # Main rendering shader (PBR, shadows, normal maps)
    shadow.wgsl          # Shadow map depth pass
    skybox.wgsl          # Procedural skybox
    lines.wgsl           # Line rendering (grid, gizmos)
  assets/                # Runtime assets
    meshes/              # 3D models (OBJ, glTF/GLB)
    textures/            # Image textures
    scenes/              # Scene files (.ron) and thumbnails (.png)
    audio/               # Sound files (.wav)
```

---

## License

*TBD*
