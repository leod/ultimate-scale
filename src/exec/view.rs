use glutin::{VirtualKeyCode, WindowEvent};

use crate::machine::Machine;
use crate::exec::Exec;
use crate::edit::Editor;
use crate::game_state::GameState;
use crate::render::{self, RenderLists};

#[derive(Debug, Clone)]
pub struct Config {
    pub pause_resume_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            pause_resume_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
        }
    }
}

pub struct ExecView {
    config: Config,
    exec: Exec,

    stop_exec: bool,
}

impl ExecView {
    pub fn new(config: Config, machine: Machine) -> ExecView {
        ExecView {
            config,
            exec: Exec::new(machine),
            stop_exec: false,
        }
    }

    pub fn update(mut self, dt_secs: f32, editor: Editor) -> GameState {
        if !self.stop_exec {
            GameState::Exec { 
                exec_view: self,
                editor: editor,
            }
        } else {
            GameState::Edit(editor)
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { device_id: _, input } =>
                self.on_keyboard_input(*input),
            _ => ()
        }
    }

    fn on_keyboard_input(&mut self, input: glutin::KeyboardInput) {
        if input.state == glutin::ElementState::Pressed {
            if let Some(keycode) = input.virtual_keycode {
                self.on_key_press(keycode);
            }
        }
    }

    fn on_key_press(&mut self, keycode: VirtualKeyCode) {
        if keycode == self.config.stop_key {
            self.stop_exec = true;
        }
    }

    pub fn render(&mut self, out: &mut RenderLists) {
        render::machine::render_machine(&self.exec.machine(), out);
        render::machine::render_xy_grid(
            &self.exec.machine().size(),
            0.01,
            &mut out.solid,
        );
    }
}
