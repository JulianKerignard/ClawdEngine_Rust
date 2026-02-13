# ClawdEngine - Feature Reference

> Moteur de jeu 3D en Rust | 7 717 lignes | wgpu 27 + egui 0.33 + winit 0.30

---

## GPU / Rendu

### Render Pipeline (Forward Dual-Pass)
- **Status** : Complet
- Architecture dual-pass : 3D scene vers texture offscreen, puis egui overlay
- 5 bind groups : Camera(0), Model(1), Material(2), Lights(3), Shadow(4)
- Format viewport : Rgba8Unorm + Depth32Float
- Rendu enregistre dans une texture egui via `register_native_texture()`
- Resize dynamique du viewport avec `update_egui_texture_from_wgpu_texture()`

### Mesh Rendering
- **Status** : Complet
- Vertex format : position(3), normal(3), uv(2), tangent(4) = 48 bytes/vertex
- Index format : u32 (32-bit)
- Topology : TriangleList avec back-face culling
- MeshStore : stockage GPU avec AABB et nommage (`builtin:cube`, `builtin:sphere`, paths OBJ)
- Maillages proceduraux : cube 1x1x1, sphere 16 lat x 24 lon

### Skybox
- **Status** : Complet
- Gradient procedural (pas de cubemap HDR)
- Single triangle fullscreen (3 vertices via vertex_index, pas de vertex buffer)
- Depth fixe z=0.9999 (toujours derriere la scene)
- 4 bandes de couleur avec smoothstep : bottom (sombre) -> horizon (bleu clair) -> mid -> top (bleu nuit)

### Viewport & Render-to-Texture
- **Status** : Complet
- Color : Rgba8Unorm | Depth : Depth32Float
- Usage color : RENDER_ATTACHMENT + TEXTURE_BINDING + COPY_SRC
- Screenshot capture : GPU readback via staging buffer + map_async, resize 256x144 Lanczos3
- Capture synchrone (stall GPU)

### Texture Management
- **Status** : Complet
- TextureStore avec cache HashMap path -> id
- Format : Rgba8UnormSrgb (sRGB)
- Sampler : Repeat (toutes directions), Linear (mag/min), Nearest mip
- Default white 1x1 (fallback albedo), default normal 1x1 [128,128,255,255]
- Chargement via crate `image`, conversion RGBA8
- **Limites** : pas de mipmapping, pas de compression, pas d'async loading

### Line Rendering (Debug / Gizmos)
- **Status** : Complet
- Thin lines (1px) : LineList topology, 4096 capacite initiale -> grid, light rays, dashes
- Thick lines (billboard quads) : TriangleList (6 vertices/segment), camera-facing -> gizmo shafts, circles, cones
- Reallocation auto si capacite depassee (next_power_of_two)
- Upload complet chaque frame via write_buffer()

### Frame Metrics
- **Status** : Complet
- Draw call count, triangle count retournes par frame
- FPS calcule par exponential smoothing

---

## Lighting

### 3 Types de Lumieres
- **Status** : Complet
- Max 4 lumieres simultanees (hard-coded)
- Encoding GPU : position.w = type (0=directional, 1=point, 2=spot), direction.w = range, spot_params = [cos_inner, cos_outer, 0, 0]

#### Directional Light
- Direction derivee de la rotation de l'entite (`rot * Vec3::new(0.0, -1.0, 0.0)`)
- Pas d'attenuation (lumiere infinie)
- Seul type qui projette des ombres

#### Point Light
- Attenuation UE4 : `falloff = pow(1 - pow(d/range, 4), 2) / (d^2 + 1)`
- Range configurable (default 10.0)
- Pas de shadow casting

#### Spot Light
- Cone avec inner/outer angle
- Attenuation : `smoothstep(cos_outer, cos_inner, theta)`
- Direction via rotation de l'entite
- Pas de shadow casting

### Ambient Light
- Couleur bleutee fixe [0.12, 0.14, 0.18] simulant le scatter atmospherique
- Appliquee uniformement a tous les objets

### Light Helpers (Visualization)
- **Status** : Complet
- Affiches uniquement quand l'entite lumiere est selectionnee
- Directional : rayons paralleles pointilles + fleche
- Point : 3 cercles orthogonaux + 6 rayons radiaux
- Spot : cone wireframe + rayon central

---

## Shadow Mapping

### Shadow Pass
- **Status** : Complet
- Depth-only pass separee (shadow.wgsl)
- Format : Depth32Float, resolution 1024x1024
- Cull front faces (previent self-shadow)
- Bias : constant=2, slope_scale=2.0

### Light View-Projection
- Orthographique : bounds [-15, 15] x [-15, 15], near 0.1, far 50.0
- Position calcul : `-light_dir * 20.0`
- Direction derivee de la rotation de la directional light

### PCF (Percentage Closer Filtering)
- 4-tap PCF : grille 2x2 avec offset 1/1024 texel
- Comparison sampler (LessEqual)
- Resultat 0.0 (ombre complete) -> 1.0 (eclaire)
- Clips hors shadow map -> full lit

### Limites
- Resolution fixe 1024x1024 (pas de cascade shadows)
- Projection orthographique fixe (pas configurable par lumiere)
- Ombre uniquement sur directional lights
- Bias hard-code

---

## Materials / PBR

### Material System
- **Status** : Complet
- Proprietes par entite (pas de multi-material par mesh)

| Propriete | Type | Default | Description |
|-----------|------|---------|-------------|
| albedo | Vec3 | (0.8, 0.8, 0.8) | Couleur diffuse |
| roughness | f32 | 0.5 | Rugosite (0=brillant, 1=mat) |
| metallic | f32 | 0.0 | Metallicite (0=dielectrique, 1=metal) |
| emission | Vec3 | (0, 0, 0) | Emission lumineuse (ajoute apres lighting) |
| texture_path | Option | None | Chemin texture albedo |
| normal_map_path | Option | None | Chemin normal map |

### Blinn-Phong Specular
- `shininess = mix(16.0, 128.0, 1.0 - roughness)`
- `fresnel = mix(vec3(0.04), base_color, metallic)`
- Specular = light.color * intensity * spec_strength * fresnel

### Normal Mapping
- **Status** : Complet
- Tangent computation : MikkTSpace-style Gram-Schmidt dans mesh.rs
- Vertex tangent : xyz direction + w handedness
- TBN matrix reconstruit en fragment shader
- Guard contre zero tangent (fallback geometric normal, evite NaN)
- Default normal : flat blue [128,128,255,255] = vec3(0,0,1) en tangent space

### Texture Bindings
- Binding 1 : albedo texture
- Binding 2 : sampler (partage)
- Binding 3 : normal map

---

## Camera

### Free Camera (FPS-style)
- **Status** : Complet
- Position directe (pas d'orbit autour d'un focus)
- Perspective : 45 deg FOV, near 0.1, far 100.0

| Controle | Action |
|----------|--------|
| Clic droit + souris | Rotation libre (yaw/pitch) |
| Clic droit + WASD | Deplacement direction de vue |
| Clic droit + E/Q | Monter / Descendre |
| Clic milieu + drag | Pan (translation laterale) |
| Scroll | Zoom (avance/recule direction de vue) |
| F | Focus sur entite selectionnee |

### Screen-to-Ray
- Conversion coords viewport -> rayon monde (origin + direction)
- Utilise pour picking entites et gizmos
- Inverse view-projection matrix

---

## ECS (Entity Component System)

### Entity Management
- **Status** : Complet
- EntityId = u32 index + u32 generation (anti-dangling reference)
- Free-list pour reutilisation de slots
- Verification `is_alive()` sur chaque acces

### World (SoA Storage)
- **Status** : Complet
- Structure of Arrays : Vec<Option<T>> par type de composant
- 5 composants standard : Transform, MeshRenderer, Material, Light, RigidBody
- TypeMap pour composants custom (`HashMap<TypeId, Vec<Option<Box<dyn Any>>>>`)
- Iterateurs : `iter_entities()`, `transforms_iter()`, `lights_iter()`, `rigid_bodies_iter()`

### Snapshot / Restore
- **Status** : Complet
- Deep clone de tous les vecteurs de composants
- Utilise pour Play mode (snapshot au start, restore au stop)
- **Bug connu** : restore() ne gere pas correctement les entity generations

---

## Physics

### Gravity + Ground Plane
- **Status** : Minimal / Stub
- Semi-implicit Euler : `velocity.y -= 9.81 * dt`, `position += velocity * dt`
- Collision ground plane y=0 uniquement (clamp + reset velocity)
- Active seulement en Play mode

### Limites
- Pas de collision sphere-sphere ou AABB
- Pas de collision mesh
- angular_velocity stocke mais non utilise
- Pas de friction, damping, constraints
- Ground plane y=0 hard-code

---

## Scripting

### GameScript Framework
- **Status** : Complet
- Trait : `name()`, `start()`, `update()`, `inspector_ui()`
- ScriptContext : wrapper securise avec acces World mutable
- Execution : `start()` une fois au Play, `update()` chaque frame

### Script Registry
- Factory pattern : `Box<dyn Fn() -> Box<dyn GameScript>>`
- Lookup par nom pour serialisation/deserialization
- Extensible depuis l'editeur

### Scripts Demo (4)

| Script | Entite | Comportement |
|--------|--------|-------------|
| RotateAndPulse | Cube | Rotation Y + cycle couleur sinusoidal |
| Oscillate | Sphere | Oscillation Y sinusoidale (amplitude + frequence) |
| GravityBounce | MetalSphere | Rebond au sol avec facteur d'amortissement |
| ColorCycle | PedestalCube | Cycle HSV -> RGB continu |

### UserScript (Configurable)
- Generic wrapper avec 5 behaviors enum (None, Rotate, Bounce, Oscillate, ColorCycle)
- Inspector UI avec combobox + sliders
- Utilise comme fallback pour scripts utilisateur

### Limites
- Pas de hot-reload
- Pas de Lua/WASM runtime
- Scripts Rust uniquement (compilation requise)
- State expose directement (pas d'encapsulation)

---

## Input

### Input System
- **Status** : Complet
- Tracking : keys held/pressed/released, mouse held/pressed/released
- Mouse position, delta, scroll delta
- Cursor initialization guard (evite delta spike au premier frame)

### Routage
- Mouse/scroll toujours route a Input (meme si egui consomme)
- Keyboard filtre si egui wants_keyboard_input
- `begin_frame()` clear pressed/released/deltas apres rendering

---

## UI / UX Editeur

### Layout Dock (4 Panneaux)
- **Status** : Complet
- egui_dock 0.18.0 avec style custom Catppuccin Mocha
- Proportions : Hierarchy (15%) | Viewport (60%) | Inspector (25%)
- Assets sous Hierarchy (40% colonne gauche)
- Onglets non fermables, non rearrangeables
- **Limite** : layout fixe, pas de save/restore

### Header / Toolbar
- **Status** : Complet
- Logo ClawdEngine (accent sapphire)
- Menu File : New Scene, Save (Ctrl+S), Load (liste scenes assets/scenes/)
- Menu View : Toggle Grid, Toggle Stats Overlay
- Boutons outils : Select(Q), Move(W), Rotate(E), Scale(R) avec highlight actif
- Play/Stop central (vert/rouge)
- Stats droite : FPS, entity count, feedback sauvegarde

### Hierarchy Panel
- **Status** : Complet
- Liste scrollable avec icones : lumiere (soleil jaune), mesh (carre bleu), vide (cercle gris)
- Selection : click simple, Shift+click toggle multi-select
- Quick Add (+) : Empty, Cube, Sphere, Light
- Context menu : Add entity, New Scene
- Delete button en bas si selection active
- **Limites** : pas de drag-drop, pas de parent-child, pas de renommage in-place

### Viewport Panel
- **Status** : Complet
- Texture 3D affichee via `ui.image(SizedTexture)`
- Stats overlay (toggle) : FPS, entities, draw calls, triangles
- Background BG_CRUST

### Inspector Panel
- **Status** : Complet
- Sections CollapsingHeader (toutes ouvertes par defaut) :

| Section | Contenu |
|---------|---------|
| Entity Name | Input texte editable |
| Transform | Position/Rotation/Scale avec axis_drag colore (XYZ = RGB) |
| Material | Color picker, roughness/metallic sliders, emission, texture/normal map browse |
| Light | Kind combobox, color, intensity, range, inner/outer angle (Spot) |
| MeshRenderer | Visible checkbox, mesh ID display |
| RigidBody | Mass, gravity toggle, velocity/angular_velocity axes |
| Scripts | Liste avec inspector_ui() custom, bouton remove |
| + Add Component | Popup : Material, MeshRenderer, RigidBody (grise si present) |
| Add Script | ComboBox depuis registry |

- Desactive en Play mode
- "Select an entity" si rien selectionne
- "N entities selected" si multi-selection

### Assets Panel
- **Status** : Complet
- Breadcrumb navigation (chaque composant cliquable)
- Vue grille icones (72x80px par item, 48x48px icone)
- Icones procedurales : Folder (dore), Script (bleu), Mesh (vert), File (gris)
- Double-click dossier -> naviguer, click .obj -> charger mesh, double-click file -> ouvrir externe
- Context menu : Delete, New Folder, New Script, New Scene
- Auto-refresh chaque frame
- **Limites** : pas de drag-drop, pas de search/filtre, pas de preview thumbnail

### Gizmo System
- **Status** : Complet
- 3 outils : Move (translation), Rotate (rotation quaternion), Scale
- 3 axes colores : X rouge, Y vert, Z bleu
- Picking : ray-line closest distance (seuil 0.15-0.2 unites)
- Drag : projection ray sur axe/plan, delta applique au Transform
- Hover highlight sur axe survole
- **Limites** : pas de snapping grid, pas de mode local/global, pas de pivot point

### Entity Picking
- **Status** : Complet
- Ray-AABB intersection (slab algorithm)
- AABB transforme avec corners du mesh
- Click viewport -> screen_to_ray -> test chaque entite visible

### Raccourcis Clavier
- **Status** : Complet

| Raccourci | Action |
|-----------|--------|
| Q / W / E / R | Select / Move / Rotate / Scale |
| Delete / Backspace | Supprimer entites selectionnees |
| Escape | Deselectionner tout |
| Shift+A | Toggle quick-add menu |
| Cmd+Z | Undo |
| Cmd+S | Save scene |
| Cmd+O | Load scene |
| Cmd+D | Dupliquer entites selectionnees |
| F | Focus camera sur selection |

- Desactives pendant fly mode (clic droit maintenu)

### Undo System
- **Status** : Complet (undo only, pas de redo)
- Stack de 32 snapshots max
- Chaque operation enregistree : spawn, delete, duplicate, add/remove component
- Snapshot = clone complet du World + selected entities
- **Limites** : pas de redo, pas de grouping d'actions, snapshots couteux (deep clone)

### Theme Catppuccin Mocha
- **Status** : Complet
- Palette sombre : BG Crust #111B -> Surface2 #585B70
- Texte : Primary #CDD6F4, Secondary #A6ADC8, Disabled #6C7086
- Accent Sapphire : #89B4FA (primary), #B4BEFE (hover), #74C7EC (pressed)
- Semantique : Success #A6E3A1, Error #F38BA8, Warning #F9E2AF
- Axes : X #DC5050, Y #50BE50, Z #5078DC
- Corner radius 4-8px, shadows window blur 12

### Icones Procedurales
- **Status** : Complet
- 48x48px generees en Rust, sauvees PNG dans icons/
- 4 types : Folder (dore, tab+body), Script (bleu, page+{}), Mesh (vert, cube wireframe), File (gris, page+lignes)
- Lazy load au premier frame, cache en TextureHandle egui

---

## Scene Management

### Scene Serialization (RON)
- **Status** : Complet
- Format : `.ron` dans `assets/scenes/`
- Preview : `.png` 256x144 (capture viewport au save)
- Contenu : entities, tous composants, scripts par nom
- Mesh reference par nom (`builtin:cube`, `builtin:sphere`, ou path OBJ)
- Deduplication noms automatique ("Scene", "Scene (1)", "Scene (2)"...)

### Default Scene (Showcase)
- **Status** : Complet
- 9 entites pre-configurees :

| Entite | Type | Details |
|--------|------|---------|
| Sun | Directional Light | Rotation -50 X / 30 Y, warm (1.0, 0.95, 0.85), intensite 1.5 |
| PointLight | Point Light | Position (-3, 3, 2), bleu froid (0.4, 0.6, 1.0), intensite 1.5 |
| WarmLight | Point Light | Position (3, 2, -2), orange chaud (1.0, 0.6, 0.2), intensite 1.2 |
| Ground | Cube scale | Scale (12, 0.1, 12), gris, roughness 0.95 |
| Cube | Cube + script | RotateAndPulse, RigidBody |
| Sphere | Sphere + script | Oscillate, vert |
| MetalSphere | Sphere + script | GravityBounce, metallic 1.0, roughness 0.1 |
| RedCube | Cube | Rouge, RigidBody |
| PedestalCube | Cube + script | ColorCycle, scale (0.8, 0.3, 0.8), tan |

### Play Mode
- **Status** : Complet
- Snapshot World au enter, restore au exit
- Scripts : start() une fois, update() chaque frame
- Physics active uniquement en Play
- Inspector desactive pendant Play

### Entity Operations
- **Status** : Complet
- Spawn : Empty, Cube, Sphere, Light (avec defaults intelligents)
- Delete : supprime composants + scripts associes
- Duplicate (Cmd+D) : clone composants, nom " (Copy)", offset +1.0 X, clone scripts via factory
- Toutes operations enregistrees dans undo stack

---

## Assets

### OBJ Loader
- **Status** : Complet
- tobj 4.0.3 avec triangulation + single index
- Extraction : positions, normales (fallback [0,1,0]), UVs (flip Y: 1.0-v)
- Tangent computation automatique (MikkTSpace-style)
- Multi-mesh par fichier OBJ supporte
- **Limites** : pas d'import material tobj, pas de skeletal animation

### Texture Loading
- Formats : PNG, JPG, JPEG
- Conversion RGBA8 automatique
- Cache par chemin (pas de double-load)

### Asset Browser Operations
- Create folder, Create script (template genere), Create scene
- Delete asset (recursif pour dossiers)
- Open externe (double-click -> commande `open` macOS)

---

## Architecture & Infra

### Stack Technique

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
| image | 0.25.5 | Image loading |
| bytemuck | 1.25.0 | GPU buffer casting |
| serde | 1.0.217 | Serialization |
| ron | 0.8.1 | Scene format |
| pollster | 0.4.0 | Async runtime leger |
| anyhow | 1.0.101 | Error handling |
| log | 0.4.25 | Logging |
| env_logger | 0.11.6 | Logging impl |

### Structure Fichiers (7 717 lignes)

```
src/
  main.rs                     482 loc  App loop
  core/
    entity.rs                  18 loc  EntityId
    components.rs             133 loc  5 composants
    world.rs                  340 loc  World SoA
  renderer/
    gpu_context.rs            615 loc  GpuContext + SceneRenderer
    scene_helpers.rs          409 loc  Helpers rendu
    mesh.rs                   255 loc  MeshStore + Vertex
    line_pipeline.rs          419 loc  LineBatch + LinePipeline
    pipeline.rs               199 loc  MeshPipeline
    shadow.rs                 229 loc  ShadowMap
    viewport.rs               181 loc  ViewportTexture
    camera.rs                 135 loc  Camera
    texture_store.rs          117 loc  TextureStore
    skybox.rs                  70 loc  SkyboxPipeline
  editor/
    pending_ops.rs            538 loc  17 operations en attente
    context.rs                320 loc  EditorContext (34 champs)
    picking.rs                284 loc  Ray picking
    gizmo_interaction.rs      223 loc  Gizmo drag/pick
    header.rs                 196 loc  Toolbar
    theme.rs                  197 loc  Catppuccin Mocha
    layout.rs                 194 loc  Dock layout
    default_scene.rs          171 loc  9 entites showcase
    icons.rs                  ~200 loc  4 icones procedurales
    shortcuts.rs               89 loc  Raccourcis clavier
    panels/
      inspector.rs            454 loc  Proprietes entite
      hierarchy.rs            132 loc  Arbre entites
      assets.rs               171 loc  Navigateur fichiers
      viewport.rs              63 loc  Canvas 3D
  scripting/                  388 loc  GameScript + 4 demos
  input/                      128 loc  Clavier/souris
  physics/                     41 loc  Gravite + ground
  assets/
    scene.rs                  194 loc  Serialization RON
    obj_loader.rs              73 loc  OBJ loading
shaders/
  mesh.wgsl                   206 loc  PBR + normal mapping
  skybox.wgsl                  56 loc  Gradient
  lines.wgsl                   30 loc  Debug lines
  shadow.wgsl                  23 loc  Depth-only
```

### Patterns Architecturaux
- **Pending Operations** : toutes mutations UI -> World passent par une file d'attente (evite borrow checker)
- **SoA ECS** : Vec<Option<T>> par type + TypeMap pour custom
- **Generational IDs** : previent dangling references apres entity destroy
- **Dual-Pass Forward** : 3D -> texture offscreen -> egui overlay
- **Lazy Loading** : icones, textures chargees au premier usage
- **Script Registry** : factory pattern pour instanciation par nom
