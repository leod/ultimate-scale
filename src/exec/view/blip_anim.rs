use nalgebra as na;

use crate::exec::{Blip, BlipDieMode, BlipSpawnMode, BlipStatus};
use crate::machine::grid::Dir3;
use crate::render;

pub fn pos_rot_anim(
    blip: Blip,
    is_on_wind: bool,
) -> pareen::AnimBox<f32, (na::Point3<f32>, na::UnitQuaternion<f32>)> {
    let center = render::machine::block_center(&blip.pos);
    let pos_anim = pareen::constant(blip.move_dir).map_or(center, move |move_dir| {
        let next_pos = blip.pos + move_dir.to_vector();
        let next_center = render::machine::block_center(&next_pos);
        pareen::lerp(center, next_center)
            .map_time_anim(move_rot_anim(blip, is_on_wind).map(|(progress, _)| progress))
    });

    let rot_anim = move_rot_anim(blip, is_on_wind).map(|(_, rot)| rot);

    pos_anim.zip(rot_anim).into_box()
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

fn move_rot_anim(
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
            spawn_move_anim(),
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
            twist_anim(blip.move_dir, move_anim()),
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
    let twist_anim = twist_anim(blip.move_dir, pareen::id() + halfway_time).map_time(|t| t * 12.0);
    //let twist_time = 1.0 / 8.0;

    // Then hold again.
    //let finish_anim = pareen::constant(na::UnitQuaternion::identity());

    // Combine all:
    let move_anim = normal_move_rot_anim(blip, true)
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
