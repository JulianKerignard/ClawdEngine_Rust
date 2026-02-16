use winit::keyboard::KeyCode;

use super::context::{EditorContext, EditorTool};
use crate::input::Input;

/// Process editor keyboard shortcuts.
/// Returns true if F-key focus was requested (caller handles camera focus).
/// `wants_keyboard` should be `egui_ctx.wants_keyboard_input()` — when true,
/// shortcuts are suppressed so text fields work normally.
pub fn handle_shortcuts(
    input: &Input,
    editor_ctx: &mut EditorContext,
    wants_keyboard: bool,
    right_mouse_held: bool,
) -> bool {
    // F5 → enter fullscreen game mode (works even in play mode)
    if input.is_key_pressed(KeyCode::F5) && !editor_ctx.fullscreen_game {
        if !editor_ctx.play_mode {
            editor_ctx.pending_play_toggle = Some(true);
        }
        editor_ctx.fullscreen_game = true;
        return false;
    }

    // ESC → exit fullscreen (priority over deselect)
    if input.is_key_pressed(KeyCode::Escape) && editor_ctx.fullscreen_game {
        editor_ctx.fullscreen_game = false;
        return false;
    }

    if editor_ctx.play_mode || wants_keyboard {
        return false;
    }

    // Cmd/Ctrl modifier
    let cmd_held = input.is_key_held(KeyCode::SuperLeft)
        || input.is_key_held(KeyCode::SuperRight)
        || input.is_key_held(KeyCode::ControlLeft)
        || input.is_key_held(KeyCode::ControlRight);

    let shift_held = input.is_key_held(KeyCode::ShiftLeft)
        || input.is_key_held(KeyCode::ShiftRight);

    // Tool switching (suppressed during fly mode and when Cmd/Ctrl held — avoids
    // conflict with Cmd+Z on AZERTY where Z physical = W on QWERTY)
    if !right_mouse_held && !cmd_held {
        if input.is_key_pressed(KeyCode::KeyQ) {
            editor_ctx.active_tool = EditorTool::Select;
        }
        if input.is_key_pressed(KeyCode::KeyW) {
            editor_ctx.active_tool = EditorTool::Move;
        }
        if input.is_key_pressed(KeyCode::KeyE) {
            editor_ctx.active_tool = EditorTool::Rotate;
        }
        if input.is_key_pressed(KeyCode::KeyR) {
            editor_ctx.active_tool = EditorTool::Scale;
        }
    }

    // Delete all selected entities
    if input.is_key_pressed(KeyCode::Delete) || input.is_key_pressed(KeyCode::Backspace) {
        if !editor_ctx.selected_entities.is_empty() {
            editor_ctx.pending_delete = editor_ctx.selected_entities.clone();
        }
    }

    // Deselect all
    if input.is_key_pressed(KeyCode::Escape) {
        editor_ctx.deselect_all();
    }

    // Shift+A → toggle quick-add menu (logical key for AZERTY)
    if input.is_char_pressed('a') && shift_held {
        editor_ctx.show_add_menu = !editor_ctx.show_add_menu;
    }

    // Cmd+Z → undo, Cmd+Shift+Z → redo (logical key for AZERTY support)
    // Block undo/redo during gizmo drag to prevent state corruption
    if cmd_held && input.is_char_pressed('z') && editor_ctx.gizmo_drag.is_none() {
        if shift_held {
            editor_ctx.pending_redo = true;
        } else {
            editor_ctx.pending_undo = true;
        }
    }

    // Cmd+S / Ctrl+S → save scene
    if cmd_held && input.is_char_pressed('s') {
        editor_ctx.pending_save_scene = Some(editor_ctx.scene_name.clone());
    }

    // Cmd+O / Ctrl+O → load scene
    if cmd_held && input.is_char_pressed('o') {
        editor_ctx.pending_load_scene = Some("scene".into());
    }

    // Cmd+D / Ctrl+D → duplicate selected entities
    if cmd_held && input.is_char_pressed('d') {
        if !editor_ctx.selected_entities.is_empty() {
            editor_ctx.pending_duplicate = editor_ctx.selected_entities.clone();
        }
    }

    // F → focus on selected entity (return true so caller can handle camera)
    if input.is_key_pressed(KeyCode::KeyF) {
        return !editor_ctx.selected_entities.is_empty();
    }

    false
}
