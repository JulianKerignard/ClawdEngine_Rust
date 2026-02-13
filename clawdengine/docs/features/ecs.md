# ECS (Entity Component System)

## Entity Management

- **Status** : Complet
- EntityId = u32 index + u32 generation (anti-dangling reference)
- Free-list pour reutilisation de slots
- Verification `is_alive()` sur chaque acces

## World (SoA Storage)

- Structure of Arrays : `Vec<Option<T>>` par type de composant
- 9 composants standard : Transform, MeshRenderer, Material, Light, RigidBody, Collider, CameraComponent, AudioSource, AudioListener
- TypeMap pour composants custom (`HashMap<TypeId, Vec<Option<Box<dyn Any>>>>`)
- Iterateurs : `iter_entities()`, `transforms_iter()`, `lights_iter()`, `rigid_bodies_iter()`
- Hierarchy parent/enfant : `set_parent()`, `remove_parent()`, `get_children()`, world transforms recursifs

## Composants

| Composant | Champs principaux |
|-----------|-------------------|
| Transform | position, rotation (Quat), scale |
| MeshRenderer | mesh_id, visible |
| Material | albedo, roughness, metallic, emission, texture_path, normal_map_path |
| Light | kind (Directional/Point/Spot), color, intensity, range, inner/outer angle |
| RigidBody | mass, gravity_enabled, velocity, angular_velocity |
| Collider | shape (Box/Sphere), center, half_extents, radius, restitution, friction, is_trigger |
| CameraComponent | fov_y, near, far, is_main |
| AudioSource | audio_path, volume, pitch, loop_audio, play_on_start, spatial, max_distance |
| AudioListener | active, volume (master) |

## Snapshot / Restore

- Deep clone de tous les vecteurs de composants
- Utilise pour Play mode (snapshot au start, restore au stop)
- Utilise pour Undo (snapshot avant chaque operation)
