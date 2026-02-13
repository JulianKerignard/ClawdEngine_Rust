# Camera

## Free Camera (FPS-style)

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

## Screen-to-Ray

- Conversion coords viewport -> rayon monde (origin + direction)
- Utilise pour picking entites, gizmos, et placement d'assets (drag-and-drop)
- Inverse view-projection matrix
- Intersection plan Y=0 pour calculer la position de drop
