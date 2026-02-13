# Lighting

## 3 Types de Lumieres

- **Status** : Complet
- Max 4 lumieres simultanees (hard-coded)
- Encoding GPU : `position.w` = type (0=directional, 1=point, 2=spot), `direction.w` = range, `spot_params` = [cos_inner, cos_outer, 0, 0]

### Directional Light
- Direction derivee de la rotation de l'entite (`rot * Vec3::new(0.0, -1.0, 0.0)`)
- Pas d'attenuation (lumiere infinie)
- Seul type qui projette des ombres

### Point Light
- Attenuation UE4 : `falloff = pow(1 - pow(d/range, 4), 2) / (d^2 + 1)`
- Range configurable (default 10.0)
- Pas de shadow casting

### Spot Light
- Cone avec inner/outer angle
- Attenuation : `smoothstep(cos_outer, cos_inner, theta)`
- Direction via rotation de l'entite
- Pas de shadow casting

## Ambient Light
- Couleur bleutee fixe [0.12, 0.14, 0.18] simulant le scatter atmospherique
- Appliquee uniformement a tous les objets

## Light Helpers (Visualization)
- Affiches uniquement quand l'entite lumiere est selectionnee
- Directional : rayons paralleles pointilles + fleche
- Point : 3 cercles orthogonaux + 6 rayons radiaux
- Spot : cone wireframe + rayon central

## Shadow Mapping

### Shadow Pass
- Depth-only pass separee (`shadow.wgsl`)
- Format : Depth32Float, resolution 1024x1024
- Cull front faces (previent self-shadow)
- Bias : constant=2, slope_scale=2.0

### Light View-Projection
- Orthographique : bounds [-15, 15] x [-15, 15], near 0.1, far 50.0
- Position : `-light_dir * 20.0`
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
