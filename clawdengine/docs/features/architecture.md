# Architecture & Infra

## Stack Technique

| Crate | Version | Role |
|-------|---------|------|
| wgpu | 27.0.1 | GPU API (Vulkan/Metal/DX12) |
| winit | 0.30.12 | Window + events |
| egui | 0.33.3 | UI immediate-mode |
| egui-wgpu | 0.33.3 | egui renderer wgpu |
| egui-winit | 0.33.3 | egui integration winit |
| egui_dock | 0.18.0 | Docking panels |
| glam | 0.31.0 | Math 3D (Vec3, Quat, Mat4) |
| tobj | 4.0.3 | OBJ loader |
| gltf | 1.4 | glTF 2.0 loader |
| image | 0.25.5 | Image loading + mipmap resize |
| bytemuck | 1.25.0 | GPU buffer casting |
| serde | 1.0.217 | Serialization |
| ron | 0.8.1 | Scene format |
| pollster | 0.4.0 | Async runtime leger |
| anyhow | 1.0.101 | Error handling |
| log | 0.4.25 | Logging |
| env_logger | 0.11.6 | Logging impl |

## Build Profiles

```toml
[profile.dev]
opt-level = 1          # Near-release speed for engine code

[profile.dev.package."*"]
opt-level = 2          # Full optimization for dependencies (image, wgpu, etc.)
```

## Structure Fichiers

```
src/
  main.rs                     App loop (ApplicationHandler)
  core/
    entity.rs                 EntityId (index + generation)
    components.rs             5 composants standard
    world.rs                  World SoA storage
  renderer/
    gpu_context.rs            GpuContext + SceneRenderer
    scene_helpers.rs          Helpers rendu
    mesh.rs                   MeshStore + Vertex
    line_pipeline.rs          LineBatch + LinePipeline
    pipeline.rs               MeshPipeline
    shadow.rs                 ShadowMap
    viewport.rs               ViewportTexture
    camera.rs                 Camera + screen_to_ray
    texture_store.rs          TextureStore + mipmaps
    skybox.rs                 SkyboxPipeline
  editor/
    asset_loader.rs           Async glTF loading state machine
    pending_ops.rs            17+ pending operations
    context.rs                EditorContext (35+ champs)
    picking.rs                Ray-AABB picking
    gizmo_interaction.rs      Gizmo drag/pick
    header.rs                 Toolbar + loading status
    theme.rs                  Catppuccin Mocha
    layout.rs                 Dock layout + component_section + property_row + texture_slot
    default_scene.rs          9 entites showcase
    icons.rs                  4 icones procedurales
    shortcuts.rs              Raccourcis clavier
    panels/
      inspector.rs            Component cards design
      hierarchy.rs            Arbre entites
      assets.rs               Navigateur fichiers + drag source
      viewport.rs             Canvas 3D + drop zone
  scripting/                  GameScript + 4 demos
  input/                      Clavier/souris
  physics/                    Gravite + ground
  assets/
    scene.rs                  Serialization RON
    obj_loader.rs             OBJ loading
    gltf_loader.rs            glTF 2.0 loading
shaders/
  mesh.wgsl                   PBR + normal mapping + shadows
  skybox.wgsl                 Gradient procedural
  lines.wgsl                  Debug lines
  shadow.wgsl                 Depth-only pass
```

## Patterns Architecturaux

- **Pending Operations** : toutes mutations UI -> World passent par une file d'attente (evite borrow checker)
- **SoA ECS** : `Vec<Option<T>>` par type + TypeMap pour custom
- **Generational IDs** : previent dangling references apres entity destroy
- **Dual-Pass Forward** : 3D -> texture offscreen -> egui overlay
- **Async Loading** : background thread (CPU) -> state machine -> main thread (GPU upload)
- **Lazy Loading** : icones, textures chargees au premier usage
- **Script Registry** : factory pattern pour instanciation par nom
- **Component Section Cards** : helper reutilisable pour tous les composants de l'inspector
