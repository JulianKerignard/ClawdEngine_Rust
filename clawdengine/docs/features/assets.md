# Assets

## OBJ Loader

- **Status** : Complet
- tobj 4.0.3 avec triangulation + single index
- Extraction : positions, normales (fallback [0,1,0]), UVs (flip Y: 1.0-v)
- Tangent computation automatique (MikkTSpace-style)
- Multi-mesh par fichier OBJ supporte
- Chargement synchrone (main thread)

## glTF 2.0 Loader

- **Status** : Complet
- Crate `gltf 1.4` pour parsing
- Supporte glTF (.gltf) et GLB (.glb) binaire
- Extraction : meshes, materials (albedo, roughness, metallic, emission), textures (albedo + normal)
- Transforms hierarchiques preserves (position, rotation, scale depuis le fichier)

## Chargement Asynchrone (glTF)

- **Status** : Complet
- State machine : `Parsing` (background thread) -> `Finalizing` (main thread GPU upload)
- Background thread : parsing glTF + precomputation mipmaps (image::resize Triangle filter)
- Main thread : upload toutes les textures en un frame (`queue.write_texture()` < 1ms chacune)
- UI non-bloquante : status "Parsing glTF..." affiche dans le header pendant le chargement
- **Optimisation** : dependencies compilees en opt-level=2 meme en dev mode

## Drag-and-Drop

- **Status** : Complet
- Source : panneau Assets, fichiers .obj/.glb/.gltf draggables (`Sense::click_and_drag` + `dnd_set_drag_payload`)
- Cible : viewport avec zone de drop invisible (`dnd_release_payload`)
- Placement intelligent : ray depuis la camera a travers le curseur, intersection plan Y=0
- Fallback : 5 unites devant la camera si le rayon est parallele au sol
- Double-clic depuis l'asset browser place a l'origine (0,0,0)

## Texture Loading

- Formats : PNG, JPG, JPEG
- Conversion RGBA8 automatique
- Cache par chemin (pas de double-load)
- Mipmapping precompute en background thread pour glTF

## Asset Browser

- Breadcrumb navigation (chaque composant cliquable)
- Vue grille icones (72x80px par item, 48x48px icone)
- Icones procedurales : Folder (dore), Script (bleu), Mesh (vert), File (gris)
- Double-clic dossier -> naviguer, double-clic mesh -> charger
- Context menu : Delete, New Folder, New Script, New Scene
- Auto-refresh chaque frame
