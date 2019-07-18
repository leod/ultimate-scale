use std::time::Duration;

use log::info;

use nalgebra as na;

use glutin::{VirtualKeyCode, WindowEvent};

use crate::edit::Editor;
use crate::exec::Exec;
use crate::game_state::GameState;
use crate::machine::grid::{Axis3, Dir3, Sign};
use crate::machine::{Block, Machine};
use crate::render::{self, RenderLists};
use crate::util::timer::Timer;

#[derive(Debug, Clone)]
pub struct Config {
    pub pause_resume_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
    pub frame_key: VirtualKeyCode,
    pub default_ticks_per_sec: f32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            pause_resume_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
            frame_key: VirtualKeyCode::F,
            default_ticks_per_sec: 8.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Playing {
        tick_timer: Timer,
    },
    Paused,
    Stopped,
}

pub struct ExecView {
    config: Config,
    exec: Exec,
    ticks_per_sec: f32,
    status: Status,
}

impl ExecView {
    pub fn new(config: &Config, machine: Machine) -> ExecView {
        ExecView {
            config: config.clone(),
            exec: Exec::new(machine),
            ticks_per_sec: config.default_ticks_per_sec,
            status: Status::Playing { tick_timer: Timer::from_hz(config.default_ticks_per_sec) },
        }
    }

    pub fn update(mut self, dt: Duration, editor: Editor) -> GameState {
        match self.status {
            Status::Playing { ref mut tick_timer } => {
                *tick_timer += dt;                

                // TODO: Run multiple ticks on lag spikes? If so, with some
                //       upper limit?
                if tick_timer.trigger_reset() {
                    self.exec.update();
                }
            }
            Status::Paused => (),
            Status::Stopped => {
                info!("Stopping exec, returning to editor");
                return GameState::Edit(editor)
            }
        }

        GameState::Exec {
            exec_view: self,
            editor,
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
        if keycode == self.config.pause_resume_key {
            match self.status {
                Status::Playing { .. } => {
                    info!("Pausing exec");
                    self.status = Status::Paused;
                }
                Status::Paused => {
                    info!("Resuming exec");

                    // Start off with running a tick so that we have immediate
                    // feedback
                    self.exec.update(); 
                    self.status = Status::Playing {
                        tick_timer: Timer::from_hz(self.ticks_per_sec),
                    };
                }
                Status::Stopped => {
                    // Should happen only if pause is pressed after stop in the
                    // same frame -- just ignore.
                }
            }
        } else if keycode == self.config.stop_key {
            self.status = Status::Stopped;
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
                        let center =  render::machine::block_center(block_pos);
                        let arrow_dir: na::Vector3<f32> = na::convert(arrow_dir);

                        let start = center + arrow_dir;
                        let end = center;

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
