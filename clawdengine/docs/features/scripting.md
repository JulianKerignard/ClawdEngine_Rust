# Scripting

## GameScript Framework

- **Status** : Complet
- Trait : `name()`, `start()`, `update()`, `inspector_ui()`
- ScriptContext : wrapper securise avec acces World mutable
- Execution : `start()` une fois au Play, `update()` chaque frame

## ScriptContext API

| Methode | Description |
|---------|-------------|
| `entity()` | EntityId du script |
| `get_transform()` / `get_transform_mut()` | Acces au Transform |
| `get_material()` / `get_material_mut()` | Acces au Material |
| `get_audio_source()` / `get_audio_source_mut()` | Acces a l'AudioSource |
| `play_audio()` | Set is_playing = true |
| `stop_audio()` | Set is_playing = false |
| `collisions()` | Iterator sur CollisionEvent du frame |

## Script Registry

- Factory pattern : `Box<dyn Fn() -> Box<dyn GameScript>>`
- Lookup par nom pour serialisation/deserialization
- Extensible depuis l'editeur

## Scripts Demo (5)

| Script | Entite | Comportement |
|--------|--------|-------------|
| RotateAndPulse | Cube | Rotation Y + cycle couleur sinusoidal |
| Oscillate | Sphere | Oscillation Y sinusoidale (amplitude + frequence) |
| GravityBounce | MetalSphere | Rebond au sol avec facteur d'amortissement |
| ColorCycle | PedestalCube | Cycle HSV -> RGB continu |
| AudioDemo | AudioChime | Play audio au start, replay toutes les 3 secondes |

## UserScript (Configurable)

- Generic wrapper avec 5 behaviors enum (None, Rotate, Bounce, Oscillate, ColorCycle)
- Inspector UI avec combobox + sliders
- Utilise comme fallback pour scripts utilisateur

## Limites

- Pas de hot-reload
- Pas de Lua/WASM runtime
- Scripts Rust uniquement (compilation requise)
- State expose directement (pas d'encapsulation)
