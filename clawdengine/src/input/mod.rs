use std::collections::HashSet;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

pub struct Input {
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    /// Logical characters pressed this frame (layout-aware, lowercase)
    chars_pressed: HashSet<char>,
    mouse_held: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    mouse_position: [f32; 2],
    mouse_delta: [f32; 2],
    scroll_delta: [f32; 2],
    prev_mouse_position: [f32; 2],
    cursor_initialized: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            chars_pressed: HashSet::new(),
            mouse_held: HashSet::new(),
            mouse_pressed: HashSet::new(),
            mouse_released: HashSet::new(),
            mouse_position: [0.0; 2],
            mouse_delta: [0.0; 2],
            scroll_delta: [0.0; 2],
            prev_mouse_position: [0.0; 2],
            cursor_initialized: false,
        }
    }

    /// Clear per-frame state. Call once at the start of each RedrawRequested.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.chars_pressed.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.mouse_delta = [0.0; 2];
        self.scroll_delta = [0.0; 2];
    }

    // ---- Event handlers (called from window_event) ----

    pub fn on_keyboard(&mut self, key: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.keys_held.insert(key) {
                    self.keys_pressed.insert(key);
                }
            }
            ElementState::Released => {
                self.keys_held.remove(&key);
                self.keys_released.insert(key);
            }
        }
    }

    pub fn on_logical_key(&mut self, ch: char, state: ElementState) {
        if state == ElementState::Pressed {
            self.chars_pressed.insert(ch);
        }
    }

    pub fn on_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.mouse_held.insert(button) {
                    self.mouse_pressed.insert(button);
                }
            }
            ElementState::Released => {
                self.mouse_held.remove(&button);
                self.mouse_released.insert(button);
            }
        }
    }

    pub fn on_cursor_moved(&mut self, x: f32, y: f32) {
        self.mouse_position = [x, y];
        if self.cursor_initialized {
            self.mouse_delta[0] += x - self.prev_mouse_position[0];
            self.mouse_delta[1] += y - self.prev_mouse_position[1];
        } else {
            self.cursor_initialized = true;
        }
        self.prev_mouse_position = [x, y];
    }

    pub fn on_scroll(&mut self, dx: f32, dy: f32) {
        self.scroll_delta[0] += dx;
        self.scroll_delta[1] += dy;
    }

    // ---- Query API ----

    pub fn is_key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    #[allow(dead_code)]
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn is_char_pressed(&self, ch: char) -> bool {
        self.chars_pressed.contains(&ch)
    }

    pub fn is_mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_held.contains(&button)
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_position(&self) -> [f32; 2] {
        self.mouse_position
    }

    pub fn mouse_delta(&self) -> [f32; 2] {
        self.mouse_delta
    }

    pub fn scroll_delta(&self) -> [f32; 2] {
        self.scroll_delta
    }
}
