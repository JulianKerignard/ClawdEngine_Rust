# ClawdEngine

A 3D game engine built from scratch in Rust, featuring a full visual editor with real-time viewport, scene management, physics, scripting, and audio.

Built for the [Claude Code hackathon](https://claude.ai/claude-code) — the entire engine was developed using Claude Code as the primary development tool.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![wgpu](https://img.shields.io/badge/wgpu-27.0.1-blue)
![egui](https://img.shields.io/badge/egui-0.33.3-green)
![License](https://img.shields.io/badge/license-MIT-lightgrey)

---

## Features

### Renderer
- **Forward dual-pass pipeline** — 3D scene rendered to offscreen texture, then composited with egui overlay
- **PBR materials** — Albedo, roughness, metallic, emission with Blinn-Phong specular
- **Normal mapping** — MikkTSpace-style tangent computation, TBN matrix reconstruction
- **Shadow mapping** — 1024x1024 depth pass with 4-tap PCF soft shadows
- **3 light types** — Directional (with shadows), Point, Spot with UE4-style attenuation
- **Skybox** — Procedural gradient rendered as fullscreen triangle
- **Line rendering** — Thin (1px) + thick (billboard quads) for debug visualization and gizmos
- **OBJ & glTF loading** — With automatic tangent generation and UV correction

### Editor
- **Dockable panels** — Hierarchy, Viewport, Inspector, Assets, Console, Game View
- **3D gizmos** — Move, Rotate, Scale with per-axis color coding (RGB = XYZ)
- **Entity picking** — Ray-AABB intersection from viewport clicks
- **Scene serialization** — RON format with automatic preview thumbnails
- **Undo/Redo** — 32-deep snapshot stack with Cmd+Z / Cmd+Shift+Z
- **Asset browser** — Grid icon view with drag-and-drop to viewport, search, breadcrumb navigation
- **Play mode** — Snapshot/restore world state, live script execution
- **Catppuccin Mocha theme** — Polished dark theme with animations and smooth transitions

### ECS & Core
- **Generational entity IDs** — u32 index + generation counter prevents dangling references
- **SoA storage** — Structure of Arrays with `Vec<Option<T>>` per component type
- **TypeMap** for custom components — `HashMap<TypeId, Vec<Option<Box<dyn Any>>>>`
- **Parent-child hierarchy** — With world transform propagation

### Physics
- **Rigid bodies** — Mass, gravity, velocity, angular velocity
- **AABB & sphere collisions** — Box-box and sphere-sphere detection with response
- **Friction, damping, restitution** — Configurable physical properties
- **Raycasting** — Ray-AABB and ray-sphere intersection queries

### Scripting
- **GameScript trait** — `start()`, `update()`, `inspector_ui()` lifecycle
- **ScriptContext** — Safe mutable access to World during script execution
- **Script registry** — Factory pattern for serialization/deserialization
- **Math utilities** — `lerp`, `smoothstep`, `remap`, `move_towards`, `rotate_towards`
- **FPS Controller** — Ready-to-use first-person character controller script

### Audio
- **rodio integration** — Spatial audio with distance attenuation
- **AudioListener pattern** — Unity-style listener entity required for audio playback
- **AudioSource** — Per-entity audio with volume, looping, spatial properties

### Game UI / HUD
- **Canvas system** — Resolution-independent UI containers with aspect ratio scaling
- **UI Elements** — Text and Panel components with anchor-based positioning
- **9 anchor points** — TopLeft, TopCenter, TopRight, etc.
- **Live preview** — UI elements rendered as overlays in both editor and game view

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021)
- A GPU supporting Vulkan, Metal, or DX12

### Build & Run

```bash
cd clawdengine
cargo run
```

The engine opens with a default showcase scene containing lights, meshes with scripts, and physics objects.

### Controls

| Input | Action |
|-------|--------|
| Right-click + drag | Rotate camera |
| Right-click + WASD | Fly camera |
| Right-click + E/Q | Move up / down |
| Middle-click + drag | Pan camera |
| Scroll wheel | Zoom |
| F | Focus on selected entity |
| Q / W / E / R | Select / Move / Rotate / Scale tool |
| Delete | Delete selected entities |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+S | Save scene |
| Cmd+D | Duplicate selection |
| Shift+A | Quick-add entity menu |
| Space | Play / Stop |

---

## Architecture

```
src/
  main.rs                 Application loop (winit ApplicationHandler)
  core/                   ECS — Entity, Components, World
  renderer/               wgpu GPU context, pipelines, mesh/texture stores
  editor/                 egui panels, gizmos, picking, theme, shortcuts
  scripting/              GameScript trait, demo scripts, FPS controller
  physics/                Rigid body, collisions, raycasting
  input/                  Keyboard & mouse state tracking
  audio/                  rodio audio system with spatial support
  assets/                 OBJ/glTF loaders, scene serialization (RON)
shaders/
  mesh.wgsl               PBR + normal mapping + shadows
  shadow.wgsl             Depth-only shadow pass
  skybox.wgsl             Procedural gradient
  lines.wgsl              Debug line rendering
```

### Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rendering | Render-to-texture + egui overlay | Decouples 3D from UI, enables Game View |
| ECS | SoA + TypeMap hybrid | Fast iteration + extensibility |
| Entity IDs | Generational u32 | Prevents dangling references |
| Scene format | RON | Human-readable, Rust-native serde |
| Game loop | Single-threaded sequential | Simplicity, no sync overhead |
| Editor state | Separate from World | Clean separation (ADR-005) |
| Mutations | Pending operations queue | Avoids borrow checker conflicts |

### Tech Stack

| Crate | Version | Purpose |
|-------|---------|---------|
| wgpu | 27.0.1 | GPU abstraction (Vulkan/Metal/DX12) |
| winit | 0.30.12 | Window management & events |
| egui | 0.33.3 | Immediate-mode UI |
| egui-wgpu | 0.33.3 | egui GPU renderer |
| egui-winit | 0.33.3 | egui input integration |
| egui_dock | 0.18.0 | Dockable panel layout |
| glam | 0.31.0 | 3D math (Vec3, Quat, Mat4) |
| tobj | 4.0.3 | OBJ mesh loading |
| gltf | 1.4 | glTF mesh loading |
| rodio | 0.21 | Audio playback |
| serde + ron | 1.0 / 0.8 | Scene serialization |
| image | 0.25.5 | Texture loading |
| bytemuck | 1.25.0 | GPU buffer casting |

---

## License

MIT

---

*Built with Claude Code during the Anthropic hackathon.*
