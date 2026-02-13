# Physics

## Gravity + Ground Plane

- **Status** : Complet
- Semi-implicit Euler : `velocity.y -= 9.81 * dt`, `position += velocity * dt`
- Collision ground plane y=0 (clamp + reset velocity)
- Active seulement en Play mode

## Collisions AABB Box-Box

- Detection par overlap des AABB transformes (world-space)
- `Collider::world_aabb()` : rotation via matrice 3x3 abs axes pour AABB conservatif
- Resolution : separation sur axe de moindre penetration
- Reponse impulsion avec restitution (coefficient moyen des deux corps)
- Supports Box et Sphere via ColliderShape enum

## Collisions Sphere-Sphere

- Detection par distance entre centres < somme des rayons
- Rayon effectif = `collider.radius * max(scale.x, scale.y, scale.z)`
- Separation proportionnelle aux masses inverses
- Impulsion elastique sur la normale de contact

## Friction, Damping, Restitution

- **Restitution** : coefficient 0.0-1.0 par collider, moyenne des deux corps a la collision
- **Friction** : composante tangentielle de l'impulsion, bornee par cone de Coulomb
- **Linear damping** : `velocity *= 0.995` chaque frame (dissipation energie)
- **Angular velocity** : stocke et serialise (preparation future)

## Collision Events

- `CollisionEvent` : entity, other, kind (Enter/Stay/Exit), normal
- Accessible via `ScriptContext::collisions()` dans les scripts
- Tracking des paires actives entre frames pour Enter/Stay/Exit

## Composant Collider

| Propriete | Type | Default | Description |
|-----------|------|---------|-------------|
| shape | ColliderShape | Box | Box ou Sphere |
| center | Vec3 | ZERO | Offset local |
| half_extents | Vec3 | (0.5, 0.5, 0.5) | Demi-taille pour Box |
| radius | f32 | 0.5 | Rayon pour Sphere |
| restitution | f32 | 0.3 | Coefficient de rebond |
| friction | f32 | 0.5 | Coefficient de friction |
| is_trigger | bool | false | Trigger (pas de reponse physique) |

## Limites

- Pas de collision mesh (convex hull, triangle mesh)
- Pas de constraints/joints
- Ground plane y=0 hard-code
- Pas de broad-phase (O(n^2) brute force)
- Pas de continuous collision detection (tunneling possible a haute vitesse)
