use std::collections::HashSet;

use glium::glutin::{ElementState, MouseButton, VirtualKeyCode, WindowEvent};

/// Keep track of pressed keys and mouse buttons.
pub struct InputState {
    /// Currently pressed keys.
    pressed_keys: HashSet<VirtualKeyCode>,

    /// Currently pressed mouse buttons.
    pressed_buttons: HashSet<MouseButton>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            pressed_buttons: HashSet::new(),
        }
    }

    /// Check if a keyboard key is currently pressed.
    pub fn is_key_pressed(&self, key_code: VirtualKeyCode) -> bool {
        self.pressed_keys.contains(&key_code)
    }

    /// Check if a mouse button is currently pressed.
    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    /// Clear any state associated with the keyboard.
    pub fn clear_keyboard(&mut self) {
        self.pressed_keys.clear();
    }

    /// Clear any state associated with the mouse.
    pub fn clear_mouse(&mut self) {
        self.pressed_buttons.clear();
    }

    /// Handle a window event to update internal state.
    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => {
                            self.pressed_keys.insert(keycode);
                        }
                        ElementState::Released => {
                            self.pressed_keys.remove(&keycode);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    self.pressed_buttons.insert(*button);
                }
                ElementState::Released => {
                    self.pressed_buttons.remove(button);
                }
            },
            _ => (),
        }
    }
}
