use std::time::Duration;

use coarse_prof::profile;
use nalgebra as na;

use glium::glutin::{self, WindowEvent};

use rendology::particle::Particle;
use rendology::{basic_obj, BasicObj, Camera, Light, RenderList};

use crate::edit::pick;
use crate::edit_camera_view::EditCameraView;
use crate::exec::anim::{AnimState, WindDeadend, WindLife};
use crate::exec::{
    Blip, BlipDieMode, BlipSpawnMode, BlipStatus, Exec, LevelProgress, LevelStatus, TickTime,
};
use crate::input_state::InputState;
use crate::machine::grid::{Dir3, Point3};
use crate::machine::{grid, BlipKind, Machine};
use crate::render;

#[derive(Debug, Clone)]
pub struct Config {}

impl Default for Config {
    fn default() -> Config {
        Config {}
    }
}

pub struct ExecView {
    exec: Exec,

    mouse_block_pos: Option<grid::Point3>,
}

impl ExecView {
    pub fn new(_config: &Config, machine: Machine) -> ExecView {
        ExecView {
            exec: Exec::new(machine, &mut rand::thread_rng()),
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
            |_| true,
        );
    }

    pub fn run_tick(&mut self) {
        profile!("tick");

        self.exec.update();
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
        render_out: &mut render::Stage,
    ) {
        assert!(
            prev_time.num_ticks_passed < time.num_ticks_passed
                || (prev_time.num_ticks_passed == time.num_ticks_passed
                    && prev_time.tick_progress() <= time.tick_progress())
        );

        let (progress_start, progress_end) = if prev_time.num_ticks_passed < time.num_ticks_passed {
            (0.0, time.tick_progress())
        } else {
            (prev_time.tick_progress(), time.tick_progress())
        };

        let sub_tick_duration = 0.005;
        let times = {
            let mut v = Vec::new();
            let mut current = (progress_start / sub_tick_duration).ceil() * sub_tick_duration;
            while current < progress_end {
                v.push(current);
                current += sub_tick_duration;
            }
            v
        };

        let mut num_spawned = 0;

        for blip in self.exec.blips().values() {
            if blip.move_dir.is_none() {
                continue;
            }

            let is_on_wind = self.is_blip_on_wind(blip);
            let pos_rot_anim = blip_pos_rot_anim(*blip, is_on_wind);

            if let Some(die_mode) = blip.status.die_mode() {
                let die_time = match die_mode {
                    BlipDieMode::PopEarly => 0.3,
                    _ => 0.8,
                };

                if die_mode != BlipDieMode::PressButton
                    && die_time >= progress_start
                    && die_time <= progress_end
                {
                    let dir: na::Vector3<f32> =
                        na::convert(blip.move_dir.map_or(na::Vector3::zeros(), Dir3::to_vector));

                    Self::kill_particles(
                        time.num_ticks_passed as f32 + die_time,
                        blip.kind,
                        &(pos_rot_anim.eval(die_time).0 + dir * 0.2),
                        &-dir,
                        &mut render_out.new_particles,
                    );

                    num_spawned += 2500;
                }
            }

            for &progress in &times {
                let spawn = match blip.status {
                    BlipStatus::Spawning(_) => progress >= 0.5,
                    BlipStatus::Existing => false,
                    BlipStatus::LiveToDie(_, BlipDieMode::PressButton) => progress >= 0.65,
                    BlipStatus::LiveToDie(_, _) => progress >= 0.5 && progress <= 0.8,
                    BlipStatus::Dying(BlipDieMode::PressButton) => progress >= 0.65,
                    BlipStatus::Dying(_) => false,
                };

                if !spawn {
                    continue;
                }

                let (pos, rot) = pos_rot_anim.eval(progress);

                let corners = [
                    na::Vector3::new(0.0, 0.0, 1.0),
                    na::Vector3::new(0.0, 0.0, -1.0),
                    na::Vector3::new(0.0, 1.0, 0.0),
                    na::Vector3::new(0.0, -1.0, 0.0),
                ];

                /*let back = rot
                    .transform_vector(&na::Vector3::new(-1.0, 0.0, 0.0))
                    .normalize();
                let side = rot.transform_vector(&na::Vector3::new(0.0, 1.0, 0.0));
                let velocity = 3.0 * side;*/

                let speed = match blip.status {
                    BlipStatus::Spawning(_) => 2.15,
                    _ => 3.0,
                };
                let friction = 9.0;

                for corner in &corners {
                    //let corner_pos = pos + rot.transform_vector(corner) * 0.04 + back * 0.05;
                    let velocity = rot.transform_vector(corner) * speed;
                    let life_duration = speed / friction;

                    let particle = Particle {
                        spawn_time: time.num_ticks_passed as f32 + progress,
                        life_duration,
                        start_pos: pos,
                        velocity,
                        color: render::machine::blip_color(blip.kind),
                        size: na::Vector2::new(0.02, 0.02),
                        friction,
                    };

                    render_out.new_particles.add(particle);
                    num_spawned += 1;
                }
            }
        }

        //println!("spawned {}", num_spawned)
    }

    fn kill_particles(
        spawn_time: f32,
        kind: BlipKind,
        pos: &na::Point3<f32>,
        tangent: &na::Vector3<f32>,
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

        for _ in 0..2500 {
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
                size: na::Vector2::new(0.013, 0.013),
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

        for &phase in &[0.0] {
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

    fn render_blips(&self, time: &TickTime, out: &mut render::Stage) {
        for (_index, blip) in self.exec.blips().iter() {
            /*let size_anim = pareen::cond(
                blip.status.is_pressing_button(),
                pareen::constant(1.0)
                    .seq(0.55, pareen::lerp(1.0, 0.9).squeeze(0.0..=0.2)).into_box(),
                1.0,
            ) * blip_size_anim(blip.status);*/
            let size_anim = blip_size_anim(blip.status);
            let is_on_wind = self.is_blip_on_wind(blip);
            let pos_rot_anim = blip_pos_rot_anim(*blip, is_on_wind);

            let size_factor = size_anim.eval(time.tick_progress());
            let (pos, rot) = pos_rot_anim.eval(time.tick_progress());

            let transform = na::Matrix4::new_translation(&pos.coords)
                * rot.to_homogeneous()
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(1.0, 0.8, 0.8));

            let color = render::machine::blip_color(blip.kind);
            let size = size_factor * 0.22;
            let params = basic_obj::Instance {
                color: na::Vector4::new(color.x, color.y, color.z, 1.0),
                transform: transform * na::Matrix4::new_scaling(size),
                ..Default::default()
            };

            render::machine::render_outline(
                &transform,
                &na::Vector3::new(size, size, size),
                1.0,
                out,
            );

            out.solid_glow[BasicObj::Cube].add(params);

            let intensity = size_factor * 20.0;
            out.lights.push(Light {
                position: pos,
                attenuation: na::Vector3::new(1.0, 6.0, 30.0),
                color: intensity * render::machine::blip_color(blip.kind),
                ..Default::default()
            });
        }
    }

    fn is_blip_on_wind(&self, blip: &Blip) -> bool {
        blip.move_dir.map_or(false, |dir| {
            self.exec
                .machine()
                .get_index(&blip.pos)
                .map_or(false, |block_index| {
                    self.exec.next_blocks().wind_out[block_index][dir]
                })
        })
    }
}

fn blip_spawn_anim() -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
    // Natural cubic spline interpolation of these points:
    //  0 0
    //  0.4 0.3
    //  0.8 1.2
    //  1 1
    //
    // Using this tool:
    //     https://tools.timodenk.com/cubic-spline-interpolation
    pareen::cubic(&[4.4034, 0.0, -4.5455e-2, 0.0])
        .switch(0.4, pareen::cubic(&[-1.2642e1, 2.0455e1, -8.1364, 1.0909]))
        .switch(
            0.8,
            pareen::cubic(&[1.6477e1, -4.9432e1, 4.7773e1, -1.3818e1]),
        )
}

fn blip_die_anim() -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
    blip_spawn_anim().backwards(1.0).map_time(|t| t * t)
}

// NOTE: Here, we use `AnimBox` instead of generics. Without this, we get HUGE
// compile times, up to 5 minutes. Apparently, with explicit types, the
// compiler's `type_length_limit` is breached. Increasing the limit helps, but
// does not fix the compile times.
//
// Of course, using `Box` everywhere probably has performance implications.
// I don't think it will matter for now, and the reduced compile times are
// worth it. However, this is not a nice situation, since it means we have to
// be careful with nested `pareen` usage.

fn blip_twist_anim(
    move_dir: Option<Dir3>,
    move_anim: pareen::Anim<impl pareen::Fun<T = f32, V = f32>>,
) -> pareen::Anim<impl pareen::Fun<T = f32, V = na::UnitQuaternion<f32>>> {
    let delta: na::Vector3<f32> =
        na::convert(move_dir.map_or(na::Vector3::zeros(), Dir3::to_vector));
    move_anim.map(move |angle| {
        na::UnitQuaternion::from_axis_angle(
            &na::Unit::new_normalize(delta),
            -angle * std::f32::consts::PI,
        )
    })
}

fn blip_size_anim(status: BlipStatus) -> pareen::AnimBox<f32, f32> {
    match status {
        BlipStatus::Spawning(mode) => {
            // Animate spawning the blip
            match mode {
                /*BlipSpawnMode::Ease =>
                pareen::constant(0.0).seq_squeeze(0.75, blip_spawn_anim()),*/
                BlipSpawnMode::Quick => blip_spawn_anim().seq_squeeze(0.5, 1.0).into_box(),
                BlipSpawnMode::Bridge => blip_spawn_anim().seq_squeeze(0.5, 1.0).into_box(),
            }
        }
        BlipStatus::Existing => pareen::constant(1.0).into_box(),
        BlipStatus::LiveToDie(spawn_mode, die_mode) => {
            blip_size_anim(BlipStatus::Spawning(spawn_mode))
                .switch(0.5, blip_size_anim(BlipStatus::Dying(die_mode)))
                .into_box()
        }
        BlipStatus::Dying(die_mode) => match die_mode {
            BlipDieMode::PopEarly => blip_die_anim().seq_squeeze(0.6, 0.0).into_box(),
            BlipDieMode::PopMiddle => pareen::constant(1.0)
                .seq_squeeze(0.65, blip_die_anim())
                .into_box(),
            BlipDieMode::PressButton => pareen::constant(1.0)
                .seq_squeeze(0.85, blip_die_anim())
                .into_box(),
        },
    }
}

fn blip_pos_rot_anim(
    blip: Blip,
    is_on_wind: bool,
) -> pareen::AnimBox<f32, (na::Point3<f32>, na::UnitQuaternion<f32>)> {
    let center = render::machine::block_center(&blip.pos);
    let pos_anim = pareen::constant(blip.move_dir).map_or(center, move |move_dir| {
        let next_pos = blip.pos + move_dir.to_vector();
        let next_center = render::machine::block_center(&next_pos);
        pareen::lerp(center, next_center)
            .map_time_anim(blip_move_rot_anim(blip, is_on_wind).map(|(progress, _)| progress))
    });

    let rot_anim = blip_move_rot_anim(blip, is_on_wind).map(|(_, rot)| rot);

    pos_anim.zip(rot_anim).into_box()
}

fn blip_move_rot_anim(
    blip: Blip,
    is_on_wind: bool,
) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    pareen::cond(
        !blip.status.is_pressing_button(),
        normal_move_rot_anim(blip, is_on_wind),
        press_button_move_rot_anim(blip),
    )
    .into_box()
}

fn blip_spawn_move_anim() -> pareen::AnimBox<f32, f32> {
    pareen::constant(0.0)
        .switch(
            0.3,
            render::machine::bridge_length_anim(0.0, 1.0, true).seq_ease_in_out(
                0.7,
                easer::functions::Quad,
                0.3,
                1.0,
            ), //.seq_continue(0.9, |length| pareen::lerp(length, 1.0).squeeze(0.0..=0.1)),
        )
        .into_box()
}

fn accelerate() -> pareen::AnimBox<f32, f32> {
    //pareen::fun(|t: f32| t.powf(4.0) - 3.0 * t.powf(3.0) + 3.0 * t.powf(2.0)).into_box()
    (pareen::id().powf(2.0f32) * 2.0)
        .switch(0.5, pareen::id())
        .into_box()
}

fn normal_move_rot_anim(
    blip: Blip,
    is_on_wind: bool,
) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    let blip = blip;
    let orient = blip.orient.to_quaternion_x();
    let next_orient = blip.next_orient().to_quaternion_x();

    // Move the blip
    let move_anim = || {
        pareen::cond(
            blip.status.is_bridge_spawning(),
            blip_spawn_move_anim(),
            pareen::cond(
                blip.is_turning(),
                pareen::constant(0.0).seq_squeeze(0.2, accelerate()),
                //pareen::constant(0.0).seq_ease_in(0.2, easer::functions::Quad, 0.6, pareen::fun(|t| t + 0.8)),
                pareen::id(),
            ),
        )
        .into_box()
    };

    // Rotate the blip
    let orient_anim = pareen::fun(move |t| {
        let rotation = blip.orient.quaternion_between(blip.next_orient());
        let next_orient = rotation * orient;
        orient
            .try_slerp(&next_orient, t, 0.001)
            .unwrap_or_else(|| next_orient.clone())
    });

    let twist_anim = || {
        pareen::cond(
            blip.status.is_spawning() || !is_on_wind,
            na::UnitQuaternion::identity(),
            blip_twist_anim(blip.move_dir, move_anim()),
        )
    };

    let rot_anim = pareen::cond(
        blip.is_turning(),
        orient_anim.seq_squeeze(0.2, twist_anim() * next_orient),
        twist_anim() * next_orient,
    );

    move_anim().zip(rot_anim).into_box()
}

fn press_button_move_rot_anim(blip: Blip) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    let move_rot_anim = normal_move_rot_anim(blip, true);
    let halfway_time = 0.55;
    let (_, hold_rot) = move_rot_anim.eval(1.0);

    // Stop in front of the button.
    let normal_anim = move_rot_anim.map(|(_, rot)| rot);

    // Quickly reset to a horizontal.
    let reach_anim = normal_move_rot_anim(blip, true)
        .map(|(_, rot)| rot)
        .map_time(move |t| halfway_time + t * 10.0);
    let reach_time = 1.0 / 20.0;

    // Then hold for a while.
    let hold_anim = pareen::constant(na::UnitQuaternion::identity());
    let hold_time = 0.15;

    // Twist frantically.
    let twist_anim =
        blip_twist_anim(blip.move_dir, pareen::id() + halfway_time).map_time(|t| t * 12.0);
    //let twist_time = 1.0 / 8.0;

    // Then hold again.
    //let finish_anim = pareen::constant(na::UnitQuaternion::identity());

    // Combine all:
    let move_anim = normal_move_rot_anim(blip, true)
        .map(|(pos, _)| pos)
        .seq_continue(halfway_time, move |halfway_pos| {
            pareen::lerp(halfway_pos, halfway_time)
                .squeeze(0.0..=0.25)
                .map(|p| p.min(0.55))
        })
        .into_box();

    //let rot_anim = twist_anim.seq_box(twist_time, finish_anim);
    let rot_anim = twist_anim;
    let rot_anim = hold_anim.seq_box(hold_time, rot_anim) * pareen::constant(hold_rot);
    let rot_anim = reach_anim.seq_box(reach_time, rot_anim);
    let rot_anim = normal_anim.seq_box(halfway_time, rot_anim);

    move_anim.zip(rot_anim).into_box()
}
