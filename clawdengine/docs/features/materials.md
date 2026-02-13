# Materials / PBR

## Material System

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

## Blinn-Phong Specular

- `shininess = mix(16.0, 128.0, 1.0 - roughness)`
- `fresnel = mix(vec3(0.04), base_color, metallic)`
- Specular = light.color * intensity * spec_strength * fresnel

## Normal Mapping

- Tangent computation : MikkTSpace-style Gram-Schmidt dans mesh.rs
- Vertex tangent : xyz direction + w handedness
- TBN matrix reconstruit en fragment shader
- Guard contre zero tangent (fallback geometric normal, evite NaN)
- Default normal : flat blue [128,128,255,255] = vec3(0,0,1) en tangent space

## Texture Bindings

- Binding 1 : albedo texture
- Binding 2 : sampler (partage)
- Binding 3 : normal map
