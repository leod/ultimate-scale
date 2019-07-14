use glutin::{VirtualKeyCode, WindowEvent};

use crate::machine::grid;
use crate::machine::Machine;
use crate::render::Camera;

pub struct Editor {
    machine: Machine,
    mouse_over_grid_pos: Option<grid::Vec3>,
}

impl Editor {
    pub fn new(size: grid::Vec3) -> Editor {
        Editor {
            machine: Machine::new(size),
            mouse_over_grid_pos: None,
        }
    }

    pub fn update(&mut self, dt_secs: f32, camera: &Camera) {
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
    }
}
