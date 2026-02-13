# Editor UI/UX

## Layout Dock (5 Panneaux)

- egui_dock 0.18.0 avec style custom Catppuccin Mocha
- Proportions : Hierarchy (15%) | Viewport (60%) | Inspector (25%)
- Assets + Console sous Hierarchy (40% colonne gauche)
- Onglets non fermables, non rearrangeables

## Header / Toolbar

- Logo ClawdEngine (accent sapphire)
- Menu File : New Scene, Save (Ctrl+S), Load (liste scenes assets/scenes/)
- Menu View : Toggle Grid, Toggle Stats Overlay
- Boutons outils : Select(Q), Move(W), Rotate(E), Scale(R) avec highlight actif
- Play/Stop central (vert/rouge)
- Stats droite : FPS, entity count, feedback sauvegarde, loading status

## Hierarchy Panel

- Liste scrollable avec icones : lumiere (soleil jaune), mesh (carre bleu), vide (cercle gris)
- Selection : click simple, Shift+click toggle multi-select
- Quick Add (+) : Empty, Cube, Sphere, Light, Camera
- Context menu : Add entity, New Scene
- Delete button en bas si selection active
- Drag-and-drop pour reparenting (parent/enfant)
- Expansion/collapse des hierarchies

## Viewport Panel

- Texture 3D affichee via `ui.image(SizedTexture)`
- Stats overlay (toggle) : FPS, entities, draw calls, triangles
- **Drag-and-drop** : zone de drop pour les assets 3D depuis le panneau Assets
- Placement au curseur : ray-plane intersection Y=0 pour positionner l'objet

## Console Panel

- Affiche les logs du moteur (info, warn, error, debug)
- Filtres par niveau de log (toggle boutons)
- Auto-scroll en bas
- Bouton Clear

## Inspector Panel

- **Design** : cartes de composants avec header colore + icone lettre + fleche collapse peinte
- Chaque composant dans un Frame style (fond BG_MANTLE, bordure, bande accent a gauche)
- Property rows avec labels a largeur fixe (72px) pour alignement
- **Interactif en Play mode** : modifiable pour tester en temps reel, restaure au stop

### Entity Header
- Icone type d'entite (lumiere/mesh/vide) + champ nom editable + ID grise

### Sections Composants

| Section | Icone | Accent | Contenu |
|---------|-------|--------|---------|
| Transform | T (bleu) | Sapphire | Position/Rotation/Scale avec axis_drag colore XYZ=RGB |
| Material | M (rose) | Pink | Surface (color, roughness, metallic), Emission, Texture slots |
| Light | L (jaune) | Warning | Kind combobox, color, intensity, range, spot cone |
| MeshRenderer | R (teal) | Teal | Visible checkbox, mesh ID monospace |
| RigidBody | P (peach) | Peach | Mass, gravity toggle, velocity/angular_velocity axes |
| Collider | C (bleu) | Accent | Shape combobox, center, half_extents/radius, restitution, friction, trigger |
| Camera | Cam (cyan) | Teal | FOV, near, far, is_main checkbox |
| AudioListener | AL (mauve) | Mauve | Active checkbox, volume slider (master) |
| AudioSource | A (jaune) | Yellow | File browser, volume, pitch, loop, play_on_start, spatial, max_distance |
| Scripts | S (vert) | Success | Liste avec inspector_ui() custom, bouton remove |

### Texture Slots (Material)
- Mini-cartes avec carre colore 32x32 (checkerboard si vide, couleur si assigne)
- Filename + bouton remove inline
- Bouton Browse popup (scanne assets/textures/)

### Audio Browser (AudioSource)
- Bouton Browse Audio popup
- Scan recursif assets/ pour fichiers .wav/.ogg
- Affichage nom fichier + bouton remove

### Boutons
- "+ Add Component" : full width, accent bleu, popup avec Material/MeshRenderer/RigidBody/Collider/Camera/AudioSource/AudioListener
- "Add Script" : ComboBox depuis registry
- Boutons remove : "x" avec highlight rouge au hover

- "Select an entity" si rien selectionne

## Gizmo System

- 3 outils : Move (translation), Rotate (rotation quaternion), Scale
- 3 axes colores : X rouge, Y vert, Z bleu
- Picking : ray-line closest distance (seuil 0.15-0.2 unites)
- Drag : projection ray sur axe/plan, delta applique au Transform
- Hover highlight sur axe survole

## Entity Picking

- Ray-AABB intersection (slab algorithm)
- AABB transforme avec corners du mesh
- Click viewport -> screen_to_ray -> test chaque entite visible

## Raccourcis Clavier

| Raccourci | Action |
|-----------|--------|
| Q / W / E / R | Select / Move / Rotate / Scale |
| Delete / Backspace | Supprimer entites selectionnees |
| Escape | Deselectionner tout |
| Shift+A | Toggle quick-add menu |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+S | Save scene |
| Cmd+O | Load scene |
| Cmd+D | Dupliquer entites selectionnees |
| F | Focus camera sur selection |

## Undo/Redo System

- Deux stacks : undo + redo, 32 entries max chacune
- Chaque operation enregistree : spawn, delete, duplicate, add/remove component
- Snapshot = clone complet du World + selected entities
- Undo (Cmd+Z) : pop undo, sauve etat actuel dans redo, restaure
- Redo (Cmd+Shift+Z) : pop redo, sauve etat actuel dans undo, restaure
- Nouvelle action → redo stack vide (standard UX)
- Limites : pas de grouping d'actions

## Theme Catppuccin Mocha

- Palette sombre : BG Crust #111B -> Surface2 #585B70
- Texte : Primary #CDD6F4, Secondary #A6ADC8, Disabled #6C7086
- Accent Sapphire : #89B4FA (primary), #B4BEFE (hover), #74C7EC (pressed)
- Semantique : Success #A6E3A1, Error #F38BA8, Warning #F9E2AF
- Axes : X #DC5050, Y #50BE50, Z #5078DC
- Corner radius 4-8px, shadows window blur 12

## Icones Procedurales

- 48x48px generees en Rust, sauvees PNG dans icons/
- 4 types : Folder (dore), Script (bleu), Mesh (vert), File (gris)
- Lazy load au premier frame, cache en TextureHandle egui
