use log::info;

use nalgebra as na;

use glutin::{VirtualKeyCode, WindowEvent};

use crate::edit::Editor;
use crate::exec::Exec;
use crate::game_state::GameState;
use crate::machine::grid::{Axis3, Dir3, Sign};
use crate::machine::{Block, Machine};
use crate::render::{self, RenderLists};

#[derive(Debug, Clone)]
pub struct Config {
    pub pause_resume_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
    pub frame_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            pause_resume_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
            frame_key: VirtualKeyCode::F,
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

    pub fn update(self, dt_secs: f32, editor: Editor) -> GameState {
        if !self.stop_exec {
            GameState::Exec {
                exec_view: self,
                editor,
            }
        } else {
            info!("Stopping exec, returning to editor");
            GameState::Edit(editor)
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.on_keyboard_input(*input),
            _ => (),
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
        } else if keycode == self.config.frame_key {
            info!("Running single frame");
            self.exec.update();
        }
    }

    pub fn render(&mut self, out: &mut RenderLists) {
        render::machine::render_machine(&self.exec.machine(), out);
        render::machine::render_xy_grid(&self.exec.machine().size(), 0.01, &mut out.solid);

        self.render_blocks(out);
        self.render_blips(out);
    }

    fn render_blocks(&self, out: &mut RenderLists) {
        let block_data = &self.exec.machine().block_data;
        let wind_state = self.exec.wind_state();

        for (index, (block_pos, placed_block)) in block_data.iter() {
            let block_wind_state = &wind_state[index];

            match placed_block.block {
                Block::PipeXY => {
                    let arrow_dir = {
                        let in_dir_a = placed_block.rotated_dir_xy(Dir3(Axis3::Y, Sign::Neg));
                        let in_dir_b = placed_block.rotated_dir_xy(Dir3(Axis3::Y, Sign::Pos));

                        match (
                            block_wind_state.wind_in(in_dir_a),
                            block_wind_state.wind_in(in_dir_b),
                        ) {
                            (true, true) => Some(na::Vector3::z()),
                            (true, false) => Some(in_dir_a.to_vector()),
                            (false, true) => Some(in_dir_b.to_vector()),
                            (false, false) => None,
                        }
                    };

                    if let Some(arrow_dir) = arrow_dir {
                        let block_pos_float: na::Point3<f32> = na::convert(*block_pos);
                        let arrow_dir_float: na::Vector3<f32> = na::convert(arrow_dir);

                        let start = block_pos_float + na::Vector3::new(0.5, 0.5, 0.3);
                        let end = start + arrow_dir_float;

                        render::machine::render_arrow(
                            &render::machine::Line {
                                start,
                                end,
                                thickness: 0.2,
                                color: na::Vector4::new(1.0, 0.0, 0.0, 1.0),
                            },
                            0.0,
                            &mut out.solid,
                        );
                    }
                }
                _ => (),
            }
        }
    }

    fn render_blips(&self, out: &mut RenderLists) {
        for (_index, blip) in self.exec.blips().iter() {
            let center = render::machine::block_center(&blip.pos); //+ 0.2f32 * na::Vector3::z();
            let transform =
                na::Matrix4::new_translation(&center.coords) * na::Matrix4::new_scaling(0.3);
            let instance = render::Instance {
                object: render::Object::Cube,
                params: render::InstanceParams {
                    color: na::Vector4::new(0.0, 1.0, 0.0, 1.0),
                    transform,
                    ..Default::default()
                },
            };

            out.solid.add_instance(&instance);
            out.solid_shadow.add_instance(&instance);
        }
    }
}
