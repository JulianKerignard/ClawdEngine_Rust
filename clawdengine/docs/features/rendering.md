# Rendering

## Render Pipeline (Forward Dual-Pass)

- **Status** : Complet
- Architecture dual-pass : 3D scene vers texture offscreen, puis egui overlay
- 5 bind groups : Camera(0), Model(1), Material(2), Lights(3), Shadow(4)
- Format viewport : Rgba8Unorm + Depth32Float
- Rendu enregistre dans une texture egui via `register_native_texture()`
- Resize dynamique du viewport avec `update_egui_texture_from_wgpu_texture()`

## Mesh Rendering

- Vertex format : position(3), normal(3), uv(2), tangent(4) = 48 bytes/vertex
- Index format : u32 (32-bit)
- Topology : TriangleList avec back-face culling
- MeshStore : stockage GPU avec AABB et nommage (`builtin:cube`, `builtin:sphere`, paths OBJ/glTF)
- Maillages proceduraux : cube 1x1x1, sphere 16 lat x 24 lon

## Skybox

- Gradient procedural (pas de cubemap HDR)
- Single triangle fullscreen (3 vertices via vertex_index, pas de vertex buffer)
- Depth fixe z=0.9999 (toujours derriere la scene)
- 4 bandes de couleur avec smoothstep : bottom (sombre) -> horizon (bleu clair) -> mid -> top (bleu nuit)

## Viewport & Render-to-Texture

- Color : Rgba8Unorm | Depth : Depth32Float
- Usage color : RENDER_ATTACHMENT + TEXTURE_BINDING + COPY_SRC
- Screenshot capture : GPU readback via staging buffer + map_async, resize 256x144 Lanczos3
- Capture synchrone (stall GPU)

## Texture Management

- TextureStore avec cache HashMap path -> id
- Format : Rgba8UnormSrgb (sRGB)
- Mipmapping : calcule en background thread via `image::resize()` (Triangle filter)
- Upload precomputed : toutes les mip levels envoyees au GPU en un seul frame
- Sampler : Repeat (toutes directions), Linear (mag/min), Nearest mip
- Default white 1x1 (fallback albedo), default normal 1x1 [128,128,255,255]

## Line Rendering (Debug / Gizmos)

- Thin lines (1px) : LineList topology, 4096 capacite initiale -> grid, light rays, dashes
- Thick lines (billboard quads) : TriangleList (6 vertices/segment), camera-facing -> gizmo shafts, circles, cones
- Reallocation auto si capacite depassee (next_power_of_two)
- Upload complet chaque frame via write_buffer()

## Frame Metrics

- Draw call count, triangle count retournes par frame
- FPS calcule par exponential smoothing
