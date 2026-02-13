# Scene Management

## Scene Serialization (RON)

- **Status** : Complet
- Format : `.ron` dans `assets/scenes/`
- Preview : `.png` 256x144 (capture viewport au save)
- Contenu : entities, tous composants (dont AudioSource, AudioListener), scripts par nom
- Mesh reference par nom (`builtin:cube`, `builtin:sphere`, ou path OBJ/glTF)
- Deduplication noms automatique ("Scene", "Scene (1)", "Scene (2)"...)

## Default Scene (Showcase)

11 entites pre-configurees :

| Entite | Type | Details |
|--------|------|---------|
| Sun | Directional Light | Rotation -50 X / 30 Y, warm (1.0, 0.95, 0.85), intensite 1.5 |
| PointLight | Point Light | Position (-3, 3, 2), bleu froid (0.4, 0.6, 1.0), intensite 1.5 |
| WarmLight | Point Light | Position (3, 2, -2), orange chaud (1.0, 0.6, 0.2), intensite 1.2 |
| Ground | Cube scale | Scale (12, 0.1, 12), gris, roughness 0.95, Collider |
| Cube | Cube + script | RotateAndPulse, RigidBody, Collider |
| Sphere | Sphere + script | Oscillate, vert |
| MetalSphere | Sphere + script | GravityBounce, metallic 1.0, roughness 0.1, RigidBody, Collider |
| RedCube | Cube | Rouge, RigidBody, Collider |
| PedestalCube | Cube + script | ColorCycle, scale (0.8, 0.3, 0.8), tan |
| AudioChime | AudioSource + script | AudioDemo, play_on_start, test_chime.wav |
| MainCamera | Camera + AudioListener | Position (0, 2, 5), FOV 60, AudioListener active |

## Play Mode

- Snapshot World au enter, restore au exit
- Scripts : start() une fois, update() chaque frame
- Physics active uniquement en Play
- Audio : `start_play_mode_audio()` active les AudioSource avec play_on_start
- **Inspector interactif** pendant Play (modifications temporaires, restaurees au stop)

## Entity Operations

- Spawn : Empty, Cube, Sphere, Light, Camera (avec defaults intelligents)
- Delete : supprime composants + scripts associes, recursif sur enfants
- Duplicate (Cmd+D) : clone tous composants (dont AudioSource, AudioListener), nom " (Copy)", offset +1.0 X, clone scripts via factory, preserve hierarchie
- Toutes operations enregistrees dans undo stack
