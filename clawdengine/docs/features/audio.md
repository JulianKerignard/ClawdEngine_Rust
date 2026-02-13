# Audio System

## Architecture

- **Status** : Complet
- Bibliotheque : rodio 0.21 (cpal backend, CoreAudio sur macOS)
- Thread audio dedie (rodio gere en interne)
- `AudioSystem` : struct avec `OutputStream`, `HashMap<u32, Sink>` par entite
- Graceful degradation : `AudioSystem::new()` retourne `Option`, moteur continue sans audio

## AudioListener (comme Unity)

- Composant requis pour entendre les sons
- Typiquement place sur la MainCamera
- `active: bool` : active/desactive l'ecoute
- `volume: f32` : volume master (0.0-1.0), multiplie tous les sons
- Sans AudioListener actif, aucun son ne joue
- Position du listener utilisee pour attenuation spatiale

## AudioSource

| Propriete | Type | Default | Description |
|-----------|------|---------|-------------|
| audio_path | Option\<String\> | None | Chemin relatif du fichier audio (.wav/.ogg) |
| volume | f32 | 1.0 | Volume individuel (0.0-1.0) |
| pitch | f32 | 1.0 | Vitesse de lecture (0.5-2.0) |
| loop_audio | bool | false | Boucle infinie |
| play_on_start | bool | false | Auto-play en mode Play |
| spatial | bool | false | Audio 3D (attenuation par distance) |
| max_distance | f32 | 20.0 | Distance max pour spatial |
| is_playing | bool | false | Etat runtime (non serialise) |

## Attenuation Spatiale

- Formule UE4 (coherente avec le lighting) :
  ```
  ratio = min(dist / max_dist, 1.0)
  falloff = (1 - ratio^4)^2 / (dist^2 + 1)
  ```
- Volume final = `source.volume * listener.volume * falloff`
- Position du listener = Transform de l'entite avec AudioListener actif

## Formats Supportes

- WAV (via symphonia-wav)
- OGG Vorbis (via symphonia-ogg)
- Decodage par Sink rodio (thread audio)

## Integration Play Mode

- Enter Play : `start_play_mode_audio()` set `is_playing=true` sur les entites avec `play_on_start`
- Chaque frame : `AudioSystem::update()` cree/maj/arrete les Sinks selon l'etat
- Exit Play : `stop_all()` arrete tout, World restaure depuis snapshot
- Hot-reload path : si `audio_path` change pendant le play, le Sink est recree

## Inspector UI

- Section jaune "AudioSource" dans l'Inspector
- Browse Audio : popup avec scan recursif assets/ pour .wav/.ogg
- Sliders volume/pitch, checkboxes loop/play_on_start/spatial
- Max Distance conditionnel (affiche si spatial=true)
- Section mauve "AudioListener" : checkbox Active + slider Volume

## Limites

- Pas de mixage avance (bus, effets, reverb)
- Pas de streaming (fichiers charges en memoire)
- Pas de preview en mode Edit (play_preview implemente mais pas expose dans l'UI)
- Pas de SpatialSink rodio (attenuation calculee manuellement via volume)
- Fichiers lourds (>10MB) peuvent causer un delai initial
