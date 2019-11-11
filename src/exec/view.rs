use std::time::Duration;

use nalgebra as na;

use glium::glutin::{self, WindowEvent};

use crate::edit::pick;
use crate::exec::anim::{WindAnimState, WindLife};
use crate::exec::{BlipStatus, Exec, TickTime};
use crate::input_state::InputState;
use crate::machine::grid::{Dir3, Point3};
use crate::machine::{grid, BlipKind, Machine};
use crate::render::pipeline::{wind, RenderLists};
use crate::render::{self, Camera, EditCameraView};

#[derive(Debug, Clone)]
pub struct Config {}

impl Default for Config {
    fn default() -> Config {
        Config {}
    }
}

pub struct ExecView {
    config: Config,
    exec: Exec,

    mouse_window_pos: na::Point2<f32>,
    mouse_block_pos: Option<grid::Point3>,
}

impl ExecView {
    pub fn new(config: &Config, machine: Machine) -> ExecView {
        ExecView {
            config: config.clone(),
            exec: Exec::new(machine, &mut rand::thread_rng()),
            mouse_window_pos: na::Point2::origin(),
            mouse_block_pos: None,
        }
    }

    pub fn update(
        &mut self,
        _dt: Duration,
        input_state: &InputState,
        camera: &Camera,
        edit_camera_view: &EditCameraView,
    ) {
        profile!("exec_view");

        self.mouse_block_pos = pick::pick_block(
            self.exec.machine(),
            camera,
            &edit_camera_view.eye(),
            &input_state.mouse_window_pos(),
        );
    }

    pub fn run_tick(&mut self) {
        profile!("tick");

        self.exec.update();
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.on_keyboard_input(*input),
            WindowEvent::MouseInput {
                state,
                button,
                modifiers,
                ..
            } => self.on_mouse_input(*state, *button, *modifiers),
            _ => (),
        }
    }

    fn on_keyboard_input(&mut self, _input: glutin::KeyboardInput) {}

    fn on_mouse_input(
        &mut self,
        state: glutin::ElementState,
        button: glutin::MouseButton,
        _modifiers: glutin::ModifiersState,
    ) {
        match button {
            glutin::MouseButton::Left if state == glutin::ElementState::Pressed => {
                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    Exec::try_spawn_blip(
                        false,
                        BlipKind::A,
                        &mouse_block_pos,
                        &self.exec.machine.blocks.indices,
                        &mut self.exec.blip_state,
                        &mut self.exec.blips,
                    );
                }
            }
            glutin::MouseButton::Right if state == glutin::ElementState::Pressed => {
                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    Exec::try_spawn_blip(
                        false,
                        BlipKind::B,
                        &mouse_block_pos,
                        &self.exec.machine.blocks.indices,
                        &mut self.exec.blip_state,
                        &mut self.exec.blips,
                    );
                }
            }
            _ => (),
        }
    }

    pub fn ui(&mut self, _ui: &imgui::Ui) {}

    pub fn render(&mut self, time: &TickTime, out: &mut RenderLists) {
        profile!("exec_view");

        render::machine::render_machine(
            &self.exec.machine(),
            time,
            Some(&self.exec),
            |_| true,
            out,
        );

        self.render_blocks(time, out);
        self.render_blips(time, out);

        if let Some(mouse_block_pos) = self.mouse_block_pos {
            assert!(self.exec.machine().is_valid_pos(&mouse_block_pos));

            let mouse_block_pos_float: na::Point3<f32> = na::convert(mouse_block_pos);

            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: mouse_block_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                    size: na::Vector3::new(1.0, 1.0, 1.0),
                },
                0.015,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut out.plain,
            );
        }
    }

    fn render_wind(
        &self,
        block_pos: &Point3,
        in_dir: Dir3,
        in_t: f32,
        out_t: f32,
        out: &mut RenderLists,
    ) {
        let block_center = render::machine::block_center(block_pos);
        let in_vector: na::Vector3<f32> = na::convert(in_dir.to_vector());

        // The cylinder object points in the direction of the x axis
        let (pitch, yaw) = in_dir.to_pitch_yaw_x();

        let transform = na::Matrix4::new_translation(&(block_center.coords + in_vector / 2.0))
            * na::Matrix4::from_euler_angles(0.0, pitch, yaw);

        let color = render::machine::wind_source_color();
        let color = na::Vector4::new(color.x, color.y, color.z, 1.0);

        for &phase in &[0.0, 0.25, 0.5, 0.75] {
            out.solid_wind.add(
                render::Object::TessellatedCylinder,
                &wind::Params {
                    transform,
                    color,
                    start: in_t,
                    end: out_t,
                    phase: 2.0 * phase * std::f32::consts::PI,
                    ..Default::default()
                },
            );
        }
    }

    fn render_blocks(&self, time: &TickTime, out: &mut RenderLists) {
        let blocks = &self.exec.machine().blocks;

        for (block_index, (block_pos, _placed_block)) in blocks.data.iter() {
            let anim_state = WindAnimState::from_exec_block(&self.exec, block_index);

            for &dir in &Dir3::ALL {
                match anim_state.wind_in(dir) {
                    WindLife::None => {}
                    WindLife::Appearing => {
                        // Interpolate, i.e. draw partial line
                        let out_t = time.tick_progress();
                        self.render_wind(block_pos, dir, 0.0, out_t, out);
                    }
                    WindLife::Existing => {
                        // Draw full line
                        self.render_wind(block_pos, dir, 0.0, 1.0, out);
                    }
                    WindLife::Disappearing => {
                        // Interpolate, i.e. draw partial line
                        let in_t = time.tick_progress();
                        self.render_wind(block_pos, dir, in_t, 1.0, out);
                    }
                }
            }
        }
    }

    fn blip_spawn_size_animation(t: f32) -> f32 {
        /*if t < 0.75 {
            0.0
        } else {
            (t - 0.75) * 4.0
        }*/

        // Periodic cubic spline interpolation of these points:
        //  0 0
        //  0.75 1.1
        //  1 1
        //
        // Using this tool:
        //     https://tools.timodenk.com/cubic-spline-interpolation
        /*if t <= 0.75 {
            -4.9778 * t.powi(3) + 5.6 * t.powi(2) + 0.06667 * t
        } else {
            14.933 * t.powi(3) - 39.2 * t.powi(2) + 33.667 * t - 8.4
        }*/

        // Natural cubic spline interpolation of these points:
        //  0 0
        //  0.4 0.3
        //  0.8 1.2
        //  1 1
        //
        // Using this tool:
        //     https://tools.timodenk.com/cubic-spline-interpolation
        if t <= 0.4 {
            4.4034 * t.powi(3) - 4.5455e-2 * t
        } else if t <= 0.8 {
            -1.2642e1 * t.powi(3) + 2.0455e1 * t.powi(2) - 8.1364 * t + 1.0909
        } else {
            1.6477e1 * t.powi(3) - 4.9432e1 * t.powi(2) + 4.7773e1 * t - 1.3818e1
        }
    }

    fn render_blips(&self, time: &TickTime, out: &mut RenderLists) {
        for (_index, blip) in self.exec.blips().iter() {
            let center = render::machine::block_center(&blip.pos);

            let size = 0.25
                * match blip.status {
                    BlipStatus::Spawning => {
                        // Animate spawning the blip
                        Self::blip_spawn_size_animation(time.tick_progress())
                    }
                    BlipStatus::Existing => 1.0,
                    BlipStatus::Dying => {
                        // Animate killing the blip
                        Self::blip_spawn_size_animation(1.0 - time.tick_progress())
                    }
                };

            // Interpolate blip position if it is moving
            let pos = if let Some(old_move_dir) = blip.old_move_dir {
                let old_pos = blip.pos - old_move_dir.to_vector();
                let old_center = render::machine::block_center(&old_pos);
                old_center + time.tick_progress() * (center - old_center)
            } else {
                center
            };

            let mut transform =
                na::Matrix4::new_translation(&pos.coords) * na::Matrix4::new_scaling(size);

            // Rotate blip if it is moving
            if let Some(old_move_dir) = blip.old_move_dir {
                let old_pos = blip.pos - old_move_dir.to_vector();
                let delta: na::Vector3<f32> = na::convert(blip.pos - old_pos);
                let angle = time.tick_progress() * std::f32::consts::PI / 2.0;
                let rot = na::Rotation3::new(delta.normalize() * angle);
                transform = transform * rot.to_homogeneous();
            }

            let color = render::machine::blip_color(blip.kind);
            let instance = render::pipeline::Instance {
                object: render::Object::Cube,
                params: render::pipeline::DefaultInstanceParams {
                    color: na::Vector4::new(color.x, color.y, color.z, 1.0),
                    transform,
                    ..Default::default()
                },
            };

            out.solid.add_instance(&instance);

            out.lights.push(render::pipeline::Light {
                position: pos,
                attenuation: na::Vector3::new(0.0, 0.0, 100.0),
                color: na::Vector3::new(0.2, 10.0, 0.5),
                radius: 10.0,
            });
        }
    }
}
