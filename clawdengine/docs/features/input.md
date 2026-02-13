# Input

## Input System

- **Status** : Complet
- Tracking : keys held/pressed/released, mouse held/pressed/released
- Mouse position, delta, scroll delta
- Cursor initialization guard (evite delta spike au premier frame)

## Routage

- Mouse/scroll toujours route a Input (meme si egui consomme)
- Keyboard filtre si egui wants_keyboard_input
- `begin_frame()` clear pressed/released/deltas apres rendering
