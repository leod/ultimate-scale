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
            default_ticks_per_sec: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Playing,
    Paused,
    Stopped,
}

pub struct ExecView {
    config: Config,
    exec: Exec,
    tick_timer: Timer,
    status: Status,
}

impl ExecView {
    pub fn new(config: &Config, machine: Machine) -> ExecView {
        ExecView {
            config: config.clone(),
            exec: Exec::new(machine),
            tick_timer: Timer::from_hz(config.default_ticks_per_sec),
            status: Status::Playing,
        }
    }

    pub fn update(mut self, dt: Duration, editor: Editor) -> GameState {
        match self.status {
            Status::Playing => {
                self.tick_timer += dt;

                // TODO: Run multiple ticks on lag spikes? If so, with some
                //       upper limit?
                if self.tick_timer.trigger_reset() {
                    self.exec.update();
                }
            }
            Status::Paused => (),
            Status::Stopped => {
                info!("Stopping exec, returning to editor");
                return GameState::Edit(editor);
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
                Status::Playing => {
                    info!("Pausing exec");
                    self.status = Status::Paused;
                }
                Status::Paused => {
                    info!("Resuming exec");
                    self.status = Status::Playing;
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
        let machine = &self.exec.machine();
        let wind_state = self.exec.wind_state();

        for (index, (block_pos, placed_block)) in machine.block_data.iter() {
            let block_wind_state = &wind_state[index];

            // Draw a wind line from all in dirs to all out dirs.
            // If there is no out dir, draw to the block center instead.
            let mut out_dirs: Vec<_> = machine
                .iter_neighbors(*block_pos)
                .filter(|(dir, index)| wind_state[*index].wind_in(dir.invert()))
                .map(|(dir, _)| Some(dir))
                .collect();

            if out_dirs.is_empty() {
                out_dirs.push(None);
            }

            for out_dir in out_dirs {
                let in_dirs = Dir3::ALL
                    .iter()
                    .filter(|dir| block_wind_state.wind_in(**dir));

                for in_dir in in_dirs {
                    let center = render::machine::block_center(block_pos);

                    let in_vector: na::Vector3<f32> = na::convert(in_dir.to_vector());
                    let out_vector: na::Vector3<f32> = out_dir
                        .map_or(na::Vector3::zeros(), |out_dir| {
                            na::convert(out_dir.to_vector())
                        });

                    let in_pos = center + in_vector / 2.0;
                    let out_pos = center + out_vector / 2.0;

                    render::machine::render_arrow(
                        &render::machine::Line {
                            start: in_pos,
                            end: out_pos,
                            thickness: 0.2,
                            color: na::Vector4::new(1.0, 0.0, 0.0, 1.0),
                        },
                        0.0,
                        &mut out.solid,
                    );
                }
            }
        }
    }

    fn render_blips(&self, out: &mut RenderLists) {
        for (_index, blip) in self.exec.blips().iter() {
            if blip.old_pos.is_none() {
                // Workaround for the fact that we use old blip positions but
                // render new machine state
                continue;
            }

            let center = render::machine::block_center(&blip.pos);
            let old_center = render::machine::block_center(&blip.old_pos.unwrap());

            let pos = old_center + self.tick_timer.progress() * (center - old_center);

            let transform =
                na::Matrix4::new_translation(&pos.coords) * na::Matrix4::new_scaling(0.3);
            let instance = render::Instance {
                object: render::Object::Cube,
                params: render::InstanceParams {
                    color: na::Vector4::new(0.0, 0.5, 0.0, 1.0),
                    transform,
                    ..Default::default()
                },
            };

            out.solid.add_instance(&instance);
            out.solid_shadow.add_instance(&instance);

            out.lights.push(render::Light {
                position: pos,
                attenuation: na::Vector3::new(0.0, 0.0, 10.0),
                color: na::Vector3::new(0.2, 1.0, 0.5),
                radius: 1.0,
            });
        }
    }
}
