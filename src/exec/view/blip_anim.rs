use std::collections::HashMap;
use std::time::Duration;

use nalgebra as na;

use crate::exec::{Blip, BlipDieMode, BlipSpawnMode, BlipStatus};
use crate::machine::grid::{self, Dir3};
use crate::render;

/// A subset of fields of `Blip` that are relevant for determining the blip's
/// animation. Most importantly, this excludes the position field. We use this
/// for caching the blip animation and prevent recomputing quaternions over and
/// over again when visualizing simulation progress within a tick.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Input {
    orient: Dir3,
    move_dir: Option<Dir3>,
    status: BlipStatus,

    // This is not actually `Blip` state, but derived from wind state.
    is_on_wind: bool,
}

impl Input {
    pub fn from_blip(blip: &Blip, is_on_wind: bool) -> Self {
        Self {
            orient: blip.orient,
            move_dir: blip.move_dir,
            status: blip.status,
            is_on_wind,
        }
    }

    pub fn next_orient(&self) -> Dir3 {
        self.move_dir.unwrap_or(self.orient)
    }

    pub fn is_turning(&self) -> bool {
        self.move_dir.map_or(false, |dir| dir != self.orient)
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Key {
    time: Duration,
    input: Input,
}

impl Key {
    pub fn at_time_f32(time_f32: f32, input: Input) -> Self {
        let time = Duration::from_secs_f32(time_f32);

        Key { time, input }
    }
}

#[derive(Clone, Debug)]
pub struct Value {
    pub isometry: na::Isometry3<f32>,
    pub isometry_mat: na::Matrix4<f32>,
    pub scaling: na::Vector3<f32>,
    pub face_dirs: [na::Vector3<f32>; 4],
}

impl Value {
    pub fn center(&self, blip_pos: &grid::Point3) -> na::Point3<f32> {
        render::machine::block_center(blip_pos) + self.isometry.translation.vector
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cache {
    // We'll use a `HashMap` for now, but this will be easy replace if we find
    // that it is a bottleneck. The number of possible values of `Input` is
    // relatively small.
    cache: HashMap<Key, Value>,
}

impl Cache {
    pub fn get_or_insert(&mut self, key: Key) -> &Value {
        self.cache
            .entry(key.clone())
            .or_insert_with(|| value_anim(key.input.clone()).eval(key.time.as_secs_f32()))
    }

    pub fn clear(&mut self) {
        log::debug!("clearing {} cached entries", self.cache.len());

        self.cache.clear();
    }
}

pub fn value_anim(input: Input) -> pareen::AnimBox<f32, Value> {
    let delta = input
        .move_dir
        .map(Dir3::to_vector)
        .unwrap_or(na::Vector3::zeros());
    let delta_f32: na::Vector3<f32> = na::convert(delta);

    let size = size_anim(input.status);
    let move_rot = move_rot_anim(input);

    move_rot
        .zip(size)
        .map(move |((move_progress, rot), size)| {
            let trans = na::Translation::from(move_progress * delta_f32);

            let face_dirs = [
                rot.transform_vector(&na::Vector3::new(0.0, 0.0, 1.0)),
                rot.transform_vector(&na::Vector3::new(0.0, 0.0, -1.0)),
                rot.transform_vector(&na::Vector3::new(0.0, 1.0, 0.0)),
                rot.transform_vector(&na::Vector3::new(0.0, -1.0, 0.0)),
            ];

            let isometry = na::Isometry3::from_parts(trans, rot);
            let isometry_mat = isometry.to_homogeneous();
            let scaling = na::Vector3::new(size, size, size);

            Value {
                isometry,
                isometry_mat,
                scaling,
                face_dirs,
            }
        })
        .into_box()
}

pub fn size_anim(status: BlipStatus) -> pareen::AnimBox<f32, f32> {
    match status {
        BlipStatus::Spawning(mode) => {
            // Animate spawning the blip
            match mode {
                /*BlipSpawnMode::Ease =>
                pareen::constant(0.0).seq_squeeze(0.75, spawn_anim()),*/
                BlipSpawnMode::Quick => spawn_anim().seq_squeeze(0.5, 1.0).into_box(),
                BlipSpawnMode::Bridge => spawn_anim().seq_squeeze(0.5, 1.0).into_box(),
            }
        }
        BlipStatus::Existing => pareen::constant(1.0).into_box(),
        BlipStatus::LiveToDie(spawn_mode, die_mode) => size_anim(BlipStatus::Spawning(spawn_mode))
            .switch(0.5, size_anim(BlipStatus::Dying(die_mode)))
            .into_box(),
        BlipStatus::Dying(die_mode) => match die_mode {
            BlipDieMode::PopEarly => die_anim().seq_squeeze(0.6, 0.0).into_box(),
            BlipDieMode::PopMiddle => pareen::constant(1.0)
                .seq_squeeze(0.65, die_anim())
                .into_box(),
            BlipDieMode::PressButton => pareen::constant(1.0)
                .seq_squeeze(0.85, die_anim())
                .into_box(),
        },
    }
}

fn spawn_anim() -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
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

fn die_anim() -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
    spawn_anim().backwards(1.0).map_time(|t| t * t)
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

fn twist_anim(
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

fn move_rot_anim(input: Input) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    pareen::cond(
        !input.status.is_pressing_button(),
        normal_move_rot_anim(input.clone()),
        press_button_move_rot_anim(input),
    )
    .into_box()
}

fn spawn_move_anim() -> pareen::AnimBox<f32, f32> {
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

fn normal_move_rot_anim(input: Input) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    // Move the blip
    let status = input.status;
    let is_turning = input.is_turning();
    let move_anim = || {
        pareen::cond(
            status.is_bridge_spawning(),
            spawn_move_anim(),
            pareen::cond(
                is_turning,
                pareen::constant(0.0).seq_squeeze(0.2, accelerate()),
                //pareen::constant(0.0).seq_ease_in(0.2, easer::functions::Quad, 0.6, pareen::fun(|t| t + 0.8)),
                pareen::id(),
            ),
        )
        .into_box()
    };

    // Rotate the blip
    let orient = input.orient.to_quaternion_x();
    let next_orient = input.next_orient().to_quaternion_x();
    let turn = input.orient.quaternion_between(input.next_orient());

    let orient_anim = pareen::fun(move |t| {
        let interp_quat = turn * orient;
        orient
            .try_slerp(&interp_quat, t, 0.001)
            .unwrap_or_else(|| next_orient)
    });

    let twist_anim = || {
        pareen::cond(
            input.status.is_spawning() || !input.is_on_wind,
            na::UnitQuaternion::identity(),
            twist_anim(input.move_dir, move_anim()),
        )
    };

    let rot_anim = pareen::cond(
        input.is_turning(),
        orient_anim.seq_squeeze(0.2, twist_anim() * next_orient),
        twist_anim() * next_orient,
    );

    move_anim().zip(rot_anim).into_box()
}

fn press_button_move_rot_anim(
    input: Input,
) -> pareen::AnimBox<f32, (f32, na::UnitQuaternion<f32>)> {
    let input_on_wind = Input {
        is_on_wind: false,
        ..input.clone()
    };

    let move_rot_anim = normal_move_rot_anim(input_on_wind.clone());

    let halfway_time = 0.55;
    let (_, hold_rot) = move_rot_anim.eval(1.0);

    // Stop in front of the button.
    let normal_anim = move_rot_anim.map(|(_, rot)| rot);

    // Quickly reset to a horizontal.
    let reach_anim = normal_move_rot_anim(input_on_wind.clone())
        .map(|(_, rot)| rot)
        .map_time(move |t| halfway_time + t * 10.0);
    let reach_time = 1.0 / 20.0;

    // Then hold for a while.
    let hold_anim = pareen::constant(na::UnitQuaternion::identity());
    let hold_time = 0.15;

    // Twist frantically.
    let twist_anim = twist_anim(input.move_dir, pareen::id() + halfway_time).map_time(|t| t * 12.0);
    //let twist_time = 1.0 / 8.0;

    // Then hold again.
    //let finish_anim = pareen::constant(na::UnitQuaternion::identity());

    // Combine all:
    let move_anim = normal_move_rot_anim(input_on_wind)
        .map(|(pos, _)| pos)
        .seq_continue(halfway_time, move |halfway_pos| {
            pareen::lerp(halfway_pos, halfway_time)
                .squeeze(0.0..=0.25)
                .map(|p| p.min(0.55f32))
        })
        .into_box();

    //let rot_anim = twist_anim.seq_box(twist_time, finish_anim);
    let rot_anim = twist_anim;
    let rot_anim = hold_anim.seq_box(hold_time, rot_anim) * pareen::constant(hold_rot);
    let rot_anim = reach_anim.seq_box(reach_time, rot_anim);
    let rot_anim = normal_anim.seq_box(halfway_time, rot_anim);

    move_anim.zip(rot_anim).into_box()
}
