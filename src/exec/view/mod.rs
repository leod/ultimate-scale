mod blip_anim;
mod event;

use std::time::Duration;

use coarse_prof::profile;
use nalgebra as na;

use glium::glutin::{self, WindowEvent};

use rendology::particle::Particle;
use rendology::{basic_obj, BasicObj, Camera, Light, RenderList};

use crate::edit::pick;
use crate::edit_camera_view::EditCameraView;
use crate::exec::anim::{AnimState, WindDeadend, WindLife};
use crate::exec::{Blip, BlipStatus, Exec, LevelProgress, LevelStatus, TickTime};
use crate::input_state::InputState;
use crate::machine::grid::{Dir3, Point3};
use crate::machine::{grid, BlipKind, Machine};
use crate::render;

use event::TransduceEvent;

#[derive(Debug, Clone)]
pub struct Config {
    particle_budget_per_tick: usize,
    close_particle_budget_fraction: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            particle_budget_per_tick: 500000,
            close_particle_budget_fraction: 0.3,
        }
    }
}

impl Config {
    fn close_particle_budget_per_tick(&self) -> usize {
        (self.particle_budget_per_tick as f32 * self.close_particle_budget_fraction) as usize
    }
}

pub struct ExecView {
    config: Config,

    exec: Exec,

    mouse_block_pos: Option<grid::Point3>,

    blip_anim_cache: blip_anim::Cache,

    transduce_events: Vec<(f32, TransduceEvent)>,
    particle_budget: Vec<f32>,
}

impl ExecView {
    pub fn new(config: &Config, machine: Machine) -> ExecView {
        ExecView {
            config: config.clone(),
            exec: Exec::new(machine, &mut rand::thread_rng()),
            mouse_block_pos: None,
            blip_anim_cache: blip_anim::Cache::default(),
            transduce_events: Vec::new(),
            particle_budget: Vec::new(),
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
            |_| true,
        );
    }

    pub fn run_tick(&mut self) {
        profile!("tick");

        self.exec.update();

        // The blip animation cache is indexed by the tick progress, among other
        // things. The tick progress offsets depend entirely on frame times, so
        // if we didn't clear the animation cache anywhere it would be allowed
        // to grow essentially without bound.
        self.blip_anim_cache.clear();
    }

    pub fn next_level_status(&self) -> LevelStatus {
        self.exec
            .next_level_progress()
            .map_or(LevelStatus::Running, LevelProgress::status)
    }

    pub fn level_progress(&self) -> Option<&LevelProgress> {
        self.exec.level_progress()
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.on_keyboard_input(*input),
            _ => (),
        }
    }

    fn on_keyboard_input(&mut self, _input: glutin::KeyboardInput) {}

    pub fn render(&mut self, time: &TickTime, out: &mut render::Stage) {
        profile!("exec_view");

        render::machine::render_machine(
            &self.exec.machine(),
            time,
            Some(&self.exec),
            |_| true,
            |_| false,
            out,
        );

        self.render_blocks(time, out);
        self.render_blips(time, out);
    }

    pub fn transduce(
        &mut self,
        prev_time: &TickTime,
        time: &TickTime,
        eye_pos: &na::Point3<f32>,
        render_out: &mut render::Stage,
    ) {
        profile!("transduce");

        assert!(
            prev_time.num_ticks_passed < time.num_ticks_passed
                || (prev_time.num_ticks_passed == time.num_ticks_passed
                    && prev_time.tick_progress() <= time.tick_progress())
        );

        let (progress_start, progress_end) = if prev_time.num_ticks_passed < time.num_ticks_passed {
            // We have jumped into a new tick.
            profile!("compute_events");
            event::compute_transduce_events(
                &self.exec,
                &self.config,
                eye_pos,
                &mut self.transduce_events,
                &mut self.particle_budget,
            );

            // Start time within tick at zero.
            (0.0, time.tick_progress())
        } else {
            // We are continuing to transduce the same tick as last update.
            (prev_time.tick_progress(), time.tick_progress())
        };

        for (event_index, (distance, event)) in self.transduce_events.iter().enumerate() {
            let budget_fraction = self.particle_budget[event_index];

            if budget_fraction == 0.0 {
                break;
            }

            let num_particles = event.num_particles(*distance);

            match event {
                TransduceEvent::BlipDeath {
                    blip_index,
                    time: die_time,
                    ..
                } => {
                    if *die_time < progress_start || *die_time > progress_end {
                        continue;
                    }

                    let blip = &self.exec.blips()[*blip_index];
                    let anim_input = self.blip_anim_input(blip);
                    let anim_value = self
                        .blip_anim_cache
                        .get_or_insert(blip_anim::Key::at_time_f32(*die_time, anim_input));

                    let dir: na::Vector3<f32> =
                        na::convert(blip.move_dir.map_or(na::Vector3::zeros(), Dir3::to_vector));

                    Self::kill_particles(
                        time.num_ticks_passed as f32 + die_time,
                        blip.kind,
                        &(anim_value.center(&blip.pos) + dir * 0.2),
                        &-dir,
                        budget_fraction,
                        &mut render_out.new_particles,
                    );
                }
                TransduceEvent::BlipSliver {
                    blip_index,
                    start_time,
                    duration,
                } => {
                    if progress_start > *start_time + *duration || *start_time > progress_end {
                        continue;
                    }

                    let blip = &self.exec.blips()[*blip_index];
                    let anim_input = self.blip_anim_input(blip);

                    let sub_tick_duration = 1.0 / (budget_fraction * num_particles as f32);
                    let mut current_time = progress_start;

                    while current_time < progress_end {
                        let anim_value =
                            self.blip_anim_cache
                                .get_or_insert(blip_anim::Key::at_time_f32(
                                    current_time,
                                    anim_input.clone(),
                                ));

                        let spawn_time = time.num_ticks_passed as f32 + current_time;
                        let speed = match blip.status {
                            BlipStatus::Spawning(_) => 2.15,
                            _ => 3.0,
                        };
                        let friction = 9.0;
                        let life_duration = speed / friction;
                        let start_pos = anim_value.center(&blip.pos);

                        for face_index in 0..4 {
                            let velocity = anim_value.face_dirs[face_index] * speed;

                            let particle = Particle {
                                spawn_time,
                                life_duration,
                                start_pos,
                                velocity,
                                color: render::machine::blip_color(blip.kind),
                                size: 0.01 * 10.0f32.sqrt(),
                                friction,
                            };

                            render_out.new_particles.add(particle);
                        }

                        current_time += sub_tick_duration;
                    }
                }
            }
        }

        /*if render_out.new_particles.as_slice().len() > 0 {
            log::info!(
                "spawned {} particles",
                render_out.new_particles.as_slice().len()
            );
        }*/
    }

    fn kill_particles(
        spawn_time: f32,
        kind: BlipKind,
        pos: &na::Point3<f32>,
        tangent: &na::Vector3<f32>,
        budget_fraction: f32,
        out: &mut RenderList<Particle>,
    ) {
        let smallest_unit =
            if tangent.x.abs() <= tangent.y.abs() && tangent.x.abs() <= tangent.z.abs() {
                na::Vector3::x()
            } else if tangent.y.abs() <= tangent.x.abs() && tangent.y.abs() <= tangent.z.abs() {
                na::Vector3::y()
            } else {
                na::Vector3::z()
            };
        let x_unit = tangent.cross(&smallest_unit).normalize();
        let y_unit = tangent.cross(&x_unit).normalize();

        let num_spawn = (500.0 * budget_fraction) as usize;
        let size_factor = (2.5 / budget_fraction).sqrt();

        for _ in 0..num_spawn {
            let radius = rand::random::<f32>() * 0.45;
            let angle = rand::random::<f32>() * std::f32::consts::PI * 2.0;

            let life_duration = rand::random::<f32>() * 0.7;
            let velocity = radius
                * (4.0 * angle.cos() * x_unit + 4.0 * angle.sin() * y_unit + tangent.normalize());

            let particle = Particle {
                spawn_time,
                life_duration,
                start_pos: *pos,
                velocity,
                color: render::machine::blip_color(kind),
                size: 0.03 * size_factor,
                friction: velocity.norm() / life_duration,
            };
            out.add(particle);
        }
    }

    fn render_wind(
        &self,
        block_pos: &Point3,
        in_dir: Dir3,
        in_t: f32,
        out_t: f32,
        out: &mut render::Stage,
    ) {
        let block_center = render::machine::block_center(block_pos);
        let in_vector: na::Vector3<f32> = na::convert(in_dir.to_vector());

        // The cylinder object points in the direction of the x axis
        let transform = na::Matrix4::new_translation(&(block_center.coords + in_vector / 2.0))
            * in_dir.invert().to_rotation_mat_x();

        for &phase in &[0.0 /*, 0.25*/] {
            out.wind.add(render::wind::Instance {
                transform,
                start: in_t,
                end: out_t,
                phase: 2.0 * phase * std::f32::consts::PI,
            });
        }
    }

    fn render_blocks(&self, time: &TickTime, out: &mut render::Stage) {
        let blocks = &self.exec.machine().blocks;

        for (block_index, (block_pos, placed_block)) in blocks.data.iter() {
            let anim_state = AnimState::from_exec_block(&self.exec, block_index);

            for &dir in &Dir3::ALL {
                // Draw half or none of the wind if it points towards a deadend
                let max = match anim_state.out_deadend[dir] {
                    Some(WindDeadend::Block) => {
                        // Don't draw wind towards block deadends
                        continue;
                    }
                    Some(WindDeadend::Space) => {
                        if !placed_block.block.is_pipe() {
                            // Don't draw wind towards deadends from non-pipes
                            continue;
                        } else {
                            0.5
                        }
                    }
                    None => 1.0,
                };

                match anim_state.wind_out[dir] {
                    WindLife::None => (),
                    WindLife::Appearing => {
                        // Interpolate, i.e. draw partial line
                        let out_t = time.tick_progress();
                        self.render_wind(block_pos, dir, 0.0, out_t.min(max), out);
                    }
                    WindLife::Existing => {
                        // Draw full line
                        self.render_wind(block_pos, dir, 0.0, 1.0f32.min(max), out);
                    }
                    WindLife::Disappearing => {
                        // Interpolate, i.e. draw partial line
                        let in_t = time.tick_progress();
                        self.render_wind(block_pos, dir, in_t.min(max), 1.0f32.min(max), out);
                    }
                }
            }
        }
    }

    fn render_blips(&mut self, time: &TickTime, out: &mut render::Stage) {
        profile!("blips");

        for (_index, blip) in self.exec.blips().iter() {
            let anim_input = self.blip_anim_input(blip);
            let anim_value = self
                .blip_anim_cache
                .get_or_insert(blip_anim::Key::at_time_f32(
                    time.tick_progress(),
                    anim_input,
                ));
            let scaling = anim_value
                .scaling
                .component_mul(&na::Vector3::new(1.0, 0.8, 0.8))
                * 0.22;

            // Shift transform to the blip's position
            let mut transform = anim_value.isometry_mat;
            transform[(0, 3)] += 0.5 + blip.pos.coords.x as f32;
            transform[(1, 3)] += 0.5 + blip.pos.coords.y as f32;
            transform[(2, 3)] += 0.5 + blip.pos.coords.z as f32;

            render::machine::render_outline(&transform, &scaling, 1.0, out);

            let color = render::machine::blip_color(blip.kind);
            let params = basic_obj::Instance {
                color: na::Vector4::new(color.x, color.y, color.z, 1.0),
                transform: transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                ..Default::default()
            };
            out.solid_glow[BasicObj::Cube].add(params);

            let intensity = anim_value.scaling.x * 10.0;
            out.lights.push(Light {
                position: anim_value.center(&blip.pos),
                //attenuation: na::Vector4::new(1.0, 6.0, 30.0, 0.0),
                attenuation: na::Vector4::new(1.0, 0.0, 0.0, 7.0),
                color: intensity * render::machine::blip_color(blip.kind),
                ..Default::default()
            });
        }
    }

    fn blip_anim_input(&self, blip: &Blip) -> blip_anim::Input {
        let is_on_wind = blip.move_dir.map_or(false, |dir| {
            self.exec
                .machine()
                .get_index(&blip.pos)
                .map_or(false, |block_index| {
                    self.exec.next_blocks().wind_out[block_index][dir]
                })
        });

        blip_anim::Input::from_blip(blip, is_on_wind)
    }
}
