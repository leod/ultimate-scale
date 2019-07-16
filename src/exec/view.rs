use glutin::{VirtualKeyCode, WindowEvent};

use crate::machine::Machine;
use crate::exec::Exec;

#[derive(Debug, Clone)]
pub struct Config {
    pub pause_resume_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            pause_resume_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::S,
        }
    }
}

pub struct ExecView {
    exec: Exec,
}

impl ExecView {
    pub fn new(machine: Machine) -> ExecView {
        ExecView {
            exec: Exec::new(machine),
        }
    }
}
