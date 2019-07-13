use nalgebra as na;

use std::collections::HashSet;

use glutin::{VirtualKeyCode, WindowEvent};

pub struct Camera {
    pub projection: na::Matrix4<f32>,
    pub view: na::Isometry3<f32>,
}

impl Camera {
    pub fn from_projection(projection: na::Matrix4<f32>) -> Camera {
        Camera {
            projection,
            view: na::Isometry3::identity(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Config {
    pub forward_key: VirtualKeyCode,
    pub left_key: VirtualKeyCode,
    pub back_key: VirtualKeyCode,
    pub right_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            forward_key: VirtualKeyCode::W,
            left_key: VirtualKeyCode::A,
            back_key: VirtualKeyCode::S,
            right_key: VirtualKeyCode::D,
        }
    }
}

pub struct Input {
    pressed_keys: HashSet<VirtualKeyCode>,
    modifiers_state: glutin::ModifiersState,
}

impl Input {
    pub fn move_camera(&self, dt_secs: f32, camera: &mut Camera) {

    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { device_id: _, input } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        glutin::ElementState::Pressed =>
                            self.pressed_keys.insert(keycode),
                        glutin::ElementState::Released =>
                            self.pressed_keys.remove(&keycode),
                    };
                }

                self.modifiers_state = input.modifiers;
            }
            _ => (),
        }
    }
}
