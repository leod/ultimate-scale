use nalgebra as na;

use rendology::{basic_obj, line, BasicObj, Light};

use crate::machine::grid::{self, Dir3, Sign};
use crate::machine::{BlipKind, Block, Machine, PlacedBlock};

use crate::exec::anim::{AnimState, WindLife};
use crate::exec::{Exec, LevelProgress, TickTime};

use crate::render::{floor, Stage};

pub const PIPE_THICKNESS: f32 = 0.04;
pub const MILL_THICKNESS: f32 = 0.2;
pub const MILL_DEPTH: f32 = 0.09;
pub const OUTLINE_THICKNESS: f32 = 6.5;
pub const OUTLINE_MARGIN: f32 = 0.000;
pub const BRIDGE_MARGIN: f32 = 0.005;
pub const BUTTON_LENGTH_MIN: f32 = 0.02;
pub const BUTTON_LENGTH_MAX: f32 = 0.055;

const GAMMA: f32 = 2.2;

pub fn gamma_correct(color: &na::Vector3<f32>) -> na::Vector3<f32> {
    na::Vector3::new(
        color.x.powf(GAMMA),
        color.y.powf(GAMMA),
        color.z.powf(GAMMA),
    )
}

pub fn wind_source_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(1.0, 0.557, 0.0))
}

pub fn wind_stripe_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(1.0, 0.325, 0.286))
}

pub fn blip_color(kind: BlipKind) -> na::Vector3<f32> {
    gamma_correct(&match kind {
        BlipKind::A => na::Vector3::new(0.0, 128.0, 255.0) / 255.0,
        BlipKind::B => na::Vector3::new(0.0, 0.737, 0.361),
    })
}

pub fn pipe_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.85, 0.85, 0.85))
}

pub fn funnel_in_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(1.0, 0.5, 0.5))
}

pub fn funnel_out_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(1.0, 1.0, 1.0))
}

pub fn inactive_blip_duplicator_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.7, 0.7, 0.7))
}

pub fn inactive_blip_wind_source_color() -> na::Vector3<f32> {
    wind_source_color()
    //na::Vector3::new(0.5, 0.0, 0.0)
}

pub fn solid_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.3, 0.2, 0.9))
}

pub fn wind_mill_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(1.0, 1.0, 1.0))
}

pub fn patient_bridge_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.95, 0.95, 0.95))
}

pub fn impatient_bridge_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.9, 0.9, 0.9))
}

pub fn button_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.8, 0.8, 0.8))
}

pub fn output_status_color(failed: bool, completed: bool) -> na::Vector3<f32> {
    gamma_correct(&if failed {
        na::Vector3::new(0.9, 0.0, 0.0)
    } else if completed {
        na::Vector3::new(0.8, 0.8, 0.8)
    } else {
        na::Vector3::new(0.3, 0.3, 0.3)
    })
}

pub fn floor_color() -> na::Vector3<f32> {
    //gamma_correct(&na::Vector3::new(0.1608, 0.4235, 0.5725))
    //gamma_correct(&na::Vector3::new(0.3, 0.3, 0.3))
    //gamma_correct(&(na::Vector3::new(52.9, 80.8, 92.2) / 255.0))
    na::Vector3::new(52.9, 80.8, 92.2) / 255.0
}

pub fn grid_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.578, 0.578, 0.578))
}

pub fn outline_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.0, 0.0, 0.0))
}

pub fn block_color(color: &na::Vector3<f32>, alpha: f32) -> na::Vector4<f32> {
    na::Vector4::new(color.x, color.y, color.z, alpha)
}

#[derive(Clone, Debug)]
pub struct Line {
    pub start: na::Point3<f32>,
    pub end: na::Point3<f32>,
    pub roll: f32,
    pub thickness: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_line(
    line: &Line,
    transform: &na::Matrix4<f32>,
    out: &mut basic_obj::RenderList<basic_obj::Instance>,
) {
    let line_start = transform.transform_point(&line.start);
    let line_end = transform.transform_point(&line.end);
    let center = line_start + (line_end - line_start) / 2.0;
    let d = line_end - line_start;

    let up = d.cross(&na::Vector3::x()) + d.cross(&na::Vector3::y()) + d.cross(&na::Vector3::z());
    let rot = na::Rotation3::new(d.normalize() * (line.roll + std::f32::consts::PI / 4.0));
    let look_at = na::Isometry3::face_towards(&center, &line_end, &(rot * up));

    let scaling = na::Vector3::new(
        line.thickness,
        line.thickness,
        (line_end - line_start).norm(),
    );
    let cube_transform = look_at.to_homogeneous() * na::Matrix4::new_nonuniform_scaling(&scaling);

    out[BasicObj::Cube].add(basic_obj::Instance {
        transform: cube_transform,
        color: line.color,
        ..Default::default()
    });
}

#[derive(Clone, Debug)]
pub struct Cuboid {
    pub center: na::Point3<f32>,
    pub size: na::Vector3<f32>,
}

#[rustfmt::skip]
pub const CUBOID_WIREFRAME_LINES: &[([isize; 3], [isize; 3])] = &[
    // Front
    ([-1, -1,  1], [ 1, -1,  1]),
    ([-1,  1,  1], [ 1,  1,  1]),
    ([-1,  1,  1], [-1, -1,  1]),
    ([ 1,  1,  1], [ 1, -1,  1]),

    // Back
    ([-1, -1, -1], [ 1, -1, -1]),
    ([-1,  1, -1], [ 1,  1, -1]),
    ([-1,  1, -1], [-1, -1, -1]),
    ([ 1,  1, -1], [ 1, -1, -1]),

    // Sides
    ([-1, -1, -1], [-1, -1,  1]),
    ([ 1, -1, -1], [ 1, -1,  1]),
    ([-1,  1, -1], [-1,  1,  1]),
    ([ 1,  1, -1], [ 1,  1,  1]),
];

pub fn render_cuboid_wireframe_with_transform(
    thickness: f32,
    color: &na::Vector4<f32>,
    transform: &na::Matrix4<f32>,
    out: &mut basic_obj::RenderList<basic_obj::Instance>,
) {
    for (start, end) in CUBOID_WIREFRAME_LINES.iter() {
        let start: na::Point3<f32> = na::convert(na::Point3::from_slice(start));
        let end: na::Point3<f32> = na::convert(na::Point3::from_slice(end));
        //let delta = (start - end).normalize();

        render_line(
            &Line {
                start: start / 2.0, //+ thickness / 2.0 * delta,
                end: end / 2.0,     //- thickness / 2.0 * delta,
                roll: 0.0,
                thickness,
                color: *color,
            },
            transform,
            out,
        );
    }
}

pub fn render_cuboid_wireframe(
    cuboid: &Cuboid,
    thickness: f32,
    color: &na::Vector4<f32>,
    out: &mut basic_obj::RenderList<basic_obj::Instance>,
) {
    let transform = na::Matrix4::new_translation(&cuboid.center.coords)
        * na::Matrix4::new_nonuniform_scaling(&cuboid.size);

    render_cuboid_wireframe_with_transform(thickness, color, &transform, out);
}

pub fn render_xy_grid(
    size: &grid::Vector3,
    z: f32,
    out: &mut rendology::RenderList<line::Instance>,
    //out: &mut basic_obj::RenderList<basic_obj::Instance>,
) {
    let color = block_color(&grid_color(), 1.0);
    let thickness = 6.5;

    for x in 0..=size.x {
        let transform = na::Matrix4::from_columns(&[
            na::Vector4::new(0.0, size.y as f32, 0.0, 0.0),
            na::Vector4::zeros(),
            na::Vector4::zeros(),
            na::Vector4::new(x as f32, 0.0, z, 1.0),
        ]);
        out.add(line::Instance {
            transform,
            color,
            thickness,
        });
    }

    for y in 0..=size.y {
        let transform = na::Matrix4::from_columns(&[
            na::Vector4::new(size.x as f32, 0.0, 0.0, 0.0),
            na::Vector4::zeros(),
            na::Vector4::zeros(),
            na::Vector4::new(0.0, y as f32, z, 1.0),
        ]);
        out.add(line::Instance {
            transform,
            color,
            thickness,
        });
    }
}

pub fn blip_spawn_scaling_anim(
    activation: Option<BlipKind>,
) -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
    // Hann window applied to sin
    /*let anim = pareen::cond(
        activation.is_some(),
        pareen::circle::<_, f32>().sin() * pareen::half_circle().sin().powf(2.0f32),
        0.0,
    );

    anim * 0.03 + 1.0*/

    let t = pareen::constant(1.0).seq(0.0, bridge_length_anim(0.0, 1.0, activation.is_some()));
    //.seq_ease_in_out(0.9, easer::functions::Circ, 0.1, 1.0);

    (-t + 0.5) * (1.0 / 15.0) + 1.0
}

pub fn bridge_length_anim(
    min: f32,
    max: f32,
    activated: bool,
) -> pareen::Anim<impl pareen::Fun<T = f32, V = f32>> {
    //pareen::cond(activated, pareen::half_circle().cos().abs(), 1.0).scale_min_max(min, max)

    // Natural cubic spline interpolation at these points:
    //  0 1
    //  0.25 0
    //  0.5 0.4
    //  0.9 0.5
    //  1.0 1
    pareen::cond(
        activated,
        pareen::cubic(&[2.6716e1, 0.0, -5.6697, 1.0])
            .switch(
                0.25,
                pareen::cubic(&[-4.3978e1, 5.3020e1, -1.8925e1, 2.1046]),
            )
            .switch(
                0.5,
                pareen::cubic(&[2.6979e1, -5.3416e1, 3.4293e1, -6.7651]),
            )
            .switch(
                0.9,
                pareen::cubic(&[-6.4762e1, 1.9429e2, -1.8864e2, 6.0115e1]),
            )
            .scale_min_max(min, max)
            .into_box(),
        max,
    )
}

pub struct Bridge {
    pub center: na::Point3<f32>,
    pub dir: Dir3,
    pub offset: f32,
    pub length: f32,
    pub size: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_bridge(bridge: &Bridge, transform: &na::Matrix4<f32>, out: &mut Stage) {
    let translation = na::Matrix4::new_translation(&bridge.center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(bridge.dir.to_vector());
    let output_transform = translation
        * transform
        * na::Matrix4::new_translation(
            &(dir_offset * (0.5 * bridge.length + bridge.offset + BRIDGE_MARGIN)),
        )
        * bridge.dir.to_rotation_mat_x();
    let scaling = na::Vector3::new(bridge.length, bridge.size, bridge.size);
    out.solid()[BasicObj::Cube].add(basic_obj::Instance {
        transform: output_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
        color: bridge.color,
        ..Default::default()
    });
    render_outline(&output_transform, &scaling, bridge.color.w, out);
}

pub struct Mill {
    pub center: na::Point3<f32>,
    pub offset: f32,
    pub length: f32,
    pub color: na::Vector4<f32>,
    pub dir: Dir3,
    pub roll: f32,
}

pub fn render_mill(mill: &Mill, transform: &na::Matrix4<f32>, out: &mut Stage) {
    let translation = na::Matrix4::new_translation(&mill.center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(mill.dir.to_vector());
    let (pitch, yaw) = mill.dir.to_pitch_yaw_x();
    let cube_transform = translation
        * transform
        * na::Matrix4::new_translation(
            &(dir_offset * (mill.length * 0.5 + mill.offset + BRIDGE_MARGIN)),
        )
        * na::Matrix4::from_euler_angles(mill.roll, pitch, yaw);
    let scaling = na::Vector3::new(mill.length, MILL_THICKNESS, MILL_DEPTH);
    out.solid()[BasicObj::Cube].add(basic_obj::Instance {
        transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
        color: mill.color,
        ..Default::default()
    });
    //render_outline(&cube_transform, &scaling, color.w, out);
}

pub struct WindMills {
    pub center: na::Point3<f32>,
    pub offset: f32,
    pub length: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_wind_mills(
    wind_mills: &WindMills,
    placed_block: &PlacedBlock,
    tick_time: &TickTime,
    anim_state: Option<&AnimState>,
    transform: &na::Matrix4<f32>,
    out: &mut Stage,
) {
    for &dir in &Dir3::ALL {
        if !placed_block.block.has_wind_hole_out(dir) {
            continue;
        }

        let roll_anim = pareen::constant(anim_state).map_or(0.0, |state| {
            let wind_time_offset = wind_mills.offset + wind_mills.length;

            let angle = || pareen::circle();

            // TODO: There is a problem with this animation in that it is
            //       faster when wind is appearing/disappearing.
            let wind_anim = pareen::anim_match!(state.wind_out[dir];
                WindLife::None => 0.0,
                WindLife::Appearing => {
                    // The wind will start moving inside of the block, so
                    // delay mill rotation until the wind reaches the
                    // outside.
                    angle().squeeze_and_surround(wind_time_offset..=1.0, 0.0)
                },
                WindLife::Existing => {
                    angle()
                },
                WindLife::Disappearing => {
                    // Stop mill rotation when wind reaches the inside of
                    // the block.
                    angle().squeeze_and_surround(0.0..=wind_time_offset, 0.0)
                },
            );

            // Only show rotation when not running into a deadend in that
            // direction.
            pareen::cond(state.out_deadend[dir].is_none(), wind_anim, 0.0)
        });

        let roll = roll_anim.eval(tick_time.tick_progress());

        for &phase in &[0.0, 0.5] {
            render_mill(
                &Mill {
                    center: wind_mills.center,
                    offset: wind_mills.offset,
                    length: wind_mills.length,
                    color: wind_mills.color,
                    dir,
                    roll: roll + phase * std::f32::consts::PI,
                },
                transform,
                out,
            );
        }
    }
}

pub fn render_half_pipe(
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    dir: Dir3,
    color: &na::Vector4<f32>,
    out: &mut basic_obj::RenderList<basic_obj::Instance>,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let scaling =
        na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.5, PIPE_THICKNESS, PIPE_THICKNESS));
    let offset = na::Matrix4::new_translation(&na::Vector3::new(-0.25, 0.0, 0.0));

    let rotation = dir.invert().to_rotation_mat_x();

    out[BasicObj::Cube].add(basic_obj::Instance {
        transform: translation * transform * rotation * offset * scaling,
        color: *color,
        ..Default::default()
    });
}

pub fn render_line_wireframe(
    thickness: f32,
    color: &na::Vector4<f32>,
    transform: &na::Matrix4<f32>,
    out: &mut Stage,
) {
    // TODO: This code is from the early prototype for outlines. Will need to
    // optimize this!

    for (start, end) in CUBOID_WIREFRAME_LINES.iter() {
        let start: na::Point3<f32> = na::convert(na::Point3::from_slice(start));
        let end: na::Point3<f32> = na::convert(na::Point3::from_slice(end));

        let line_start = transform.transform_point(&(start * 0.5));
        let line_end = transform.transform_point(&(end * 0.5));
        let d = line_end - line_start;
        let line_transform = na::Matrix4::from_columns(&[
            na::Vector4::new(d.x, d.y, d.z, 0.0),
            na::Vector4::zeros(),
            na::Vector4::zeros(),
            na::Vector4::new(line_start.x, line_start.y, line_start.z, 1.0),
        ]);

        out.lines.add(line::Instance {
            transform: line_transform,
            color: *color,
            thickness,
        });
    }
}

pub fn render_outline(
    cube_transform: &na::Matrix4<f32>,
    scaling: &na::Vector3<f32>,
    alpha: f32,
    out: &mut Stage,
) {
    let transform = cube_transform
        * na::Matrix4::new_nonuniform_scaling(
            &(scaling + na::Vector3::new(OUTLINE_MARGIN, OUTLINE_MARGIN, OUTLINE_MARGIN)),
        );

    let alpha = alpha * if out.dither { 0.3 } else { 0.6 };

    render_line_wireframe(
        OUTLINE_THICKNESS,
        &block_color(&outline_color(), alpha),
        &transform,
        out,
    );
}

pub fn render_pulsator(
    tick_time: &TickTime,
    anim_state: Option<&AnimState>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: &na::Vector4<f32>,
    out: &mut Stage,
) {
    let have_flow = anim_state.map_or(false, |anim| anim.num_alive_out() > 0);

    let max_size = 3.5 * PIPE_THICKNESS;
    let size_anim = pareen::cond(
        have_flow,
        pareen::half_circle().sin().powi(2) * 0.08f32 + 1.0,
        1.0,
    ) * max_size;

    let size = size_anim.eval(tick_time.tick_progress());

    let translation = na::Matrix4::new_translation(&center.coords);
    let cube_transform = translation * transform;
    let scaling = na::Vector3::new(size, size, size);

    out.solid()[BasicObj::Cube].add(basic_obj::Instance {
        transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
        color: *color,
        ..Default::default()
    });

    render_outline(&cube_transform, &scaling, color.w, out);
}

pub fn render_block(
    placed_block: &PlacedBlock,
    tick_time: &TickTime,
    anim_state: Option<&AnimState>,
    level_progress: Option<&LevelProgress>,
    next_level_progress: Option<&LevelProgress>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    alpha: f32,
    out: &mut Stage,
) {
    let translation = na::Matrix4::new_translation(&center.coords);

    match placed_block.block {
        Block::Pipe(dir_a, dir_b) => {
            let color = block_color(&pipe_color(), alpha);

            render_half_pipe(center, transform, dir_a, &color, out.solid());
            render_half_pipe(center, transform, dir_b, &color, out.solid());

            // Pulsator to hide our shame of wind direction change
            if dir_a.0 != dir_b.0 {
                render_pulsator(tick_time, anim_state, center, transform, &color, out);
            }
        }
        Block::PipeMergeXY => {
            let color = block_color(&pipe_color(), alpha);
            let scaling = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                PIPE_THICKNESS,
                1.0,
                PIPE_THICKNESS,
            ));

            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: translation * transform * scaling,
                color,
                ..Default::default()
            });

            let rot_transform = transform
                * na::Matrix4::new_rotation(na::Vector3::z() * std::f32::consts::PI / 2.0);
            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: translation * rot_transform * scaling,
                color,
                ..Default::default()
            });

            render_pulsator(tick_time, anim_state, center, transform, &color, out);
        }
        Block::GeneralPipe(ref dirs) => {
            let color = block_color(&pipe_color(), alpha);

            for (dir, &enabled) in dirs.iter() {
                if enabled {
                    render_half_pipe(center, transform, dir, &color, out.solid());
                }
            }

            // Pulsator to hide our shame of wind direction change. Only needed
            // for non-straight pipes
            if !grid::is_straight(dirs) {
                render_pulsator(tick_time, anim_state, center, transform, &color, out);
            }
        }
        Block::FunnelXY { flow_dir } => {
            let cube_transform = translation
                * transform
                * flow_dir.invert().to_rotation_mat_x()
                * na::Matrix4::new_translation(&na::Vector3::new(0.1, 0.0, 0.0));
            let scaling = na::Vector3::new(0.7, 0.45, 0.45);

            out.solid_dither[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                color: block_color(&funnel_in_color(), alpha * 0.7),
                ..Default::default()
            });
            render_outline(&cube_transform, &scaling, alpha, out);

            let input_size = 0.3;
            let input_transform = translation
                * transform
                * flow_dir.invert().to_rotation_mat_x()
                * na::Matrix4::new_translation(&na::Vector3::new(-0.4, 0.0, 0.0));
            let scaling = &na::Vector3::new(0.3, input_size, input_size);
            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: input_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                color: block_color(&funnel_out_color(), alpha),
                ..Default::default()
            });
            render_outline(&input_transform, &scaling, alpha, out);

            let pipe_color = block_color(&pipe_color(), alpha);

            render_half_pipe(center, transform, flow_dir, &pipe_color, out.solid());
            render_half_pipe(
                center,
                transform,
                flow_dir.invert(),
                &pipe_color,
                out.solid(),
            );
        }
        Block::WindSource => {
            let cube_transform = translation * transform;
            let scaling = na::Vector3::new(0.6, 0.6, 0.6);

            let render_list = if anim_state.is_some() {
                &mut out.solid_glow
            } else {
                out.solid()
            };
            render_list[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                color: block_color(&wind_source_color(), alpha),
                ..Default::default()
            });

            render_outline(&cube_transform, &scaling, alpha, out);

            if anim_state.is_some() {
                out.lights.push(Light {
                    position: *center,
                    attenuation: na::Vector3::new(1.0, 0.0, 3.0),
                    color: 8.0 * wind_source_color(),
                    ..Default::default()
                });
            }

            render_wind_mills(
                &WindMills {
                    center: *center,
                    offset: 0.3,
                    length: 0.1,
                    color: block_color(&wind_mill_color(), alpha),
                },
                placed_block,
                tick_time,
                anim_state,
                transform,
                out,
            );
        }
        Block::BlipSpawn {
            out_dir,
            kind,
            num_spawns,
        } => {
            let activation = anim_state.and_then(|s| s.activation);
            let scaling_anim = blip_spawn_scaling_anim(activation);

            let cube_color = block_color(&blip_color(kind), alpha);
            let cube_transform = translation
                * transform
                * out_dir.to_rotation_mat_x()
                * na::Matrix4::new_translation(&na::Vector3::new(-0.25 / 2.0, 0.0, 0.0));

            let size_anim =
                scaling_anim.as_ref() * pareen::constant(na::Vector3::new(0.5, 0.6, 0.6));
            let size = size_anim.eval(tick_time.tick_progress());

            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&size),
                color: cube_color,
                ..Default::default()
            });

            render_outline(&cube_transform, &size, alpha, out);

            let bridge_size_anim =
                pareen::cond(num_spawns.is_some(), 0.15, 0.25) * scaling_anim.as_ref();
            let bridge_length_anim = bridge_length_anim(0.05, 0.4, activation.is_some());

            let bridge_anim = bridge_size_anim.zip(bridge_length_anim).zip(size).map(
                |((bridge_size, bridge_length), size)| Bridge {
                    center: *center,
                    dir: out_dir,
                    offset: size.x / 2.0 - 0.25 / 2.0,
                    length: bridge_length,
                    size: bridge_size,
                    color: block_color(&patient_bridge_color(), alpha),
                },
            );

            render_bridge(&bridge_anim.eval(tick_time.tick_progress()), transform, out);
        }
        Block::BlipDuplicator { out_dirs, kind, .. } => {
            let cube_transform = translation * transform * out_dirs.0.to_rotation_mat_x();
            let activation = anim_state.and_then(|s| s.activation);
            let next_activation = anim_state.and_then(|s| s.next_activation);
            let kind_color = activation.map_or_else(inactive_blip_duplicator_color, blip_color);

            let scaling_anim = blip_spawn_scaling_anim(activation);
            let size_anim =
                scaling_anim.as_ref() * pareen::constant(na::Vector3::new(0.45, 0.6, 0.6));
            let size = size_anim.eval(tick_time.tick_progress());

            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&size),
                color: block_color(&kind_color, alpha),
                ..Default::default()
            });
            render_outline(&cube_transform, &size, alpha, out);

            let bridge_length =
                bridge_length_anim(0.05, 0.3, activation.is_some()).eval(tick_time.tick_progress());
            let button_size = (scaling_anim.as_ref() * 0.25).eval(tick_time.tick_progress());

            for &dir in &[out_dirs.0, out_dirs.1] {
                render_bridge(
                    &Bridge {
                        center: *center,
                        dir,
                        offset: size.x / 2.0,
                        length: bridge_length,
                        size: button_size,
                        color: block_color(&impatient_bridge_color(), alpha),
                    },
                    transform,
                    out,
                );
            }

            let button_length = pareen::cond(
                next_activation.is_some(),
                pareen::constant(activation.is_some())
                    .seq(1.0 - size.y / 2.0, next_activation.is_some()),
                activation.is_some(),
            )
            .map(|a| {
                if a {
                    BUTTON_LENGTH_MIN
                } else {
                    BUTTON_LENGTH_MAX
                }
            })
            .eval(tick_time.tick_progress());

            let button_color = kind.map_or(button_color(), blip_color);

            for &dir in &Dir3::ALL {
                if dir == out_dirs.0 || dir == out_dirs.1 {
                    continue;
                }

                render_bridge(
                    &Bridge {
                        center: *center,
                        dir,
                        offset: size.y / 2.0,
                        length: button_length,
                        size: button_size,
                        color: block_color(&button_color, alpha),
                    },
                    transform,
                    out,
                );
            }
        }
        Block::BlipWindSource { button_dir } => {
            let activation = anim_state.and_then(|s| s.activation.as_ref());
            let next_activation = anim_state.and_then(|s| s.next_activation.as_ref());

            let cube_color = block_color(
                &if activation.is_some() {
                    wind_source_color()
                } else {
                    inactive_blip_wind_source_color()
                },
                alpha,
            );

            let render_list = if activation.is_some() {
                &mut out.solid_glow
            } else {
                out.solid()
            };

            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0));
            let scaling = na::Vector3::new(0.6, 0.6, 0.6);
            render_list[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                color: cube_color,
                ..Default::default()
            });
            render_outline(&cube_transform, &scaling, alpha, out);

            if activation.is_some() {
                out.lights.push(Light {
                    position: *center,
                    attenuation: na::Vector3::new(1.0, 0.0, 3.0),
                    color: 8.0 * wind_source_color(),
                    ..Default::default()
                });
            }

            let button_length_anim = pareen::cond(
                next_activation.is_some(),
                pareen::constant(activation.is_some()).seq(0.85, next_activation.is_some()),
                activation.is_some(),
            )
            .map(|a| {
                if a {
                    BUTTON_LENGTH_MIN
                } else {
                    BUTTON_LENGTH_MAX
                }
            });

            render_bridge(
                &Bridge {
                    center: *center,
                    dir: button_dir,
                    offset: 0.6 / 2.0,
                    length: button_length_anim.eval(tick_time.tick_progress()),
                    size: 0.4,
                    color: block_color(&button_color(), alpha),
                },
                transform,
                out,
            );

            render_wind_mills(
                &WindMills {
                    center: *center,
                    offset: 0.6 / 2.0,
                    length: 0.1,
                    color: block_color(&wind_mill_color(), alpha),
                },
                placed_block,
                tick_time,
                anim_state,
                transform,
                out,
            );
        }
        Block::Solid => {
            let cube_transform = translation * transform;
            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform,
                color: block_color(&solid_color(), alpha),
                ..Default::default()
            });
            render_outline(
                &cube_transform,
                &na::Vector3::new(1.0, 1.0, 1.0),
                alpha,
                out,
            );
        }
        Block::Input { out_dir, .. } => {
            let is_wind_active = anim_state
                .as_ref()
                .map_or(false, |anim| anim.wind_out[Dir3::X_POS].is_alive());
            let active_blip_kind = anim_state.and_then(|anim| anim.activation);

            let angle_anim = pareen::cond(is_wind_active, pareen::half_circle(), 0.0)
                + std::f32::consts::PI / 4.0;
            let angle = angle_anim.eval(tick_time.tick_progress());

            let rotation = na::Matrix4::from_euler_angles(angle, 0.0, 0.0);

            let color = block_color(
                &active_blip_kind.map_or(na::Vector3::new(0.3, 0.3, 0.3), blip_color),
                alpha,
            );

            let cube_transform = translation * transform * rotation;
            let scaling = na::Vector3::new(0.8, 0.6, 0.6);
            out.solid()[BasicObj::Cube].add(basic_obj::Instance {
                transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                color,
                ..Default::default()
            });
            render_outline(&cube_transform, &scaling, alpha, out);

            let bridge_length = bridge_length_anim(0.1, 0.35, active_blip_kind.is_some())
                .eval(tick_time.tick_progress());

            render_bridge(
                &Bridge {
                    center: *center,
                    dir: out_dir,
                    offset: 0.4,
                    length: bridge_length,
                    size: 0.3,
                    color: block_color(&patient_bridge_color(), alpha),
                },
                transform,
                out,
            );
        }
        Block::Output { in_dir, index, .. } => {
            render_half_pipe(
                center,
                transform,
                in_dir,
                &block_color(&pipe_color(), alpha),
                &mut out.solid,
            );
            render_half_pipe(
                &(center + na::Vector3::new(0.0, 0.0, PIPE_THICKNESS / 2.0)),
                transform,
                Dir3::Z_NEG,
                &block_color(&pipe_color(), alpha),
                &mut out.solid,
            );

            let status_color = |progress: Option<&LevelProgress>| {
                let (failed, completed) = progress
                    .and_then(|progress| {
                        progress.outputs.get(index).map(|output| {
                            let num_expected = progress.inputs_outputs.outputs[index].len();
                            let completed = output.num_fed == num_expected;

                            (output.failed, completed)
                        })
                    })
                    .unwrap_or((false, false));

                output_status_color(failed, completed)
            };

            let expected_output =
                level_progress.and_then(|progress| progress.expected_output(index));
            let next_expected_output =
                next_level_progress.and_then(|next_progress| next_progress.expected_output(index));

            let expected_color_anim = pareen::constant(expected_output)
                .seq(0.6, next_expected_output)
                .map(|kind| kind.map_or(impatient_bridge_color(), blip_color))
                .map(|color| block_color(&color, alpha));

            let status_color_anim = pareen::constant(status_color(level_progress))
                .seq(0.45, status_color(next_level_progress))
                .map(|color| block_color(&color, alpha));

            let thingy_translation =
                na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, -0.3));
            let thingy_scaling =
                na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.2, 0.2, 0.4));
            out.solid_glow[BasicObj::Cube].add(basic_obj::Instance {
                transform: translation * thingy_translation * transform * thingy_scaling,
                color: expected_color_anim.eval(tick_time.tick_progress()),
                ..Default::default()
            });

            let floor_translation = na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, -0.5));
            let floor_scaling =
                na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.8, 0.8, 0.15));
            out.solid[BasicObj::Cube].add(basic_obj::Instance {
                transform: translation * floor_translation * transform * floor_scaling,
                color: status_color_anim.eval(tick_time.tick_progress()),
                ..Default::default()
            });
        }
        Block::DetectorBlipDuplicator {
            out_dir,
            flow_axis,
            kind,
            ..
        } => {
            let activation = anim_state.and_then(|s| s.activation.as_ref());
            let kind_color = match activation.or(kind.as_ref()) {
                Some(kind) => blip_color(*kind),
                None => inactive_blip_duplicator_color(),
            };
            let pipe_color = block_color(&pipe_color(), alpha);

            render_half_pipe(
                center,
                transform,
                Dir3(flow_axis, Sign::Neg),
                &pipe_color,
                &mut out.solid,
            );
            render_half_pipe(
                center,
                transform,
                Dir3(flow_axis, Sign::Pos),
                &pipe_color,
                &mut out.solid,
            );
            render_half_pipe(center, transform, out_dir, &pipe_color, &mut out.solid);

            let render_list = if activation.is_some() {
                &mut out.solid_glow
            } else {
                &mut out.solid
            };
            render_cuboid_wireframe(
                &Cuboid {
                    center: *center,
                    size: na::Vector3::new(0.7, 0.7, 0.7),
                },
                0.1,
                &block_color(&kind_color, alpha),
                render_list,
            );
        }
        Block::Air => (),
    }
}

pub fn block_center(pos: &grid::Point3) -> na::Point3<f32> {
    let coords_float: na::Vector3<f32> = na::convert(pos.coords);
    na::Point3::from(coords_float) + na::Vector3::new(0.5, 0.5, 0.5)
}

pub fn placed_block_transform(_placed_block: &PlacedBlock) -> na::Matrix4<f32> {
    //na::Matrix4::new_rotation(placed_block.angle_xy_radians() * na::Vector3::z())
    na::Matrix4::identity()
}

pub fn render_pillar(machine: &Machine, pos: &grid::Point3, alpha: f32, out: &mut Stage) {
    let mut cur = *pos;

    while cur.z > 0 {
        let mut next = cur;
        next.z -= 1;

        let is_free = |p| machine.get(&p).map_or(true, |block| block.block.is_air());

        while is_free(next) && next.z >= 0 {
            next.z -= 1;
        }

        let height = cur.z - next.z - 1;

        if height > 0 && next.z >= -1 {
            let start: na::Vector3<f32> = na::convert(cur.coords);
            let center = start + na::Vector3::new(0.5, 0.5, -height as f32 / 2.0);
            let transform = na::Matrix4::new_translation(&center)
                * na::Matrix4::from_columns(&[
                    na::Vector4::new(0.0, 0.0, 1.0, 0.0),
                    na::Vector4::new(0.0, 1.0, 0.0, 0.0),
                    na::Vector4::new(1.0, 0.0, 0.0, 0.0),
                    na::Vector4::new(0.0, 0.0, 0.0, 1.0),
                ])
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                    -height as f32,
                    0.05,
                    0.05,
                ));

            out.solid()[basic_obj::BasicObj::TessellatedCylinder].add(basic_obj::Instance {
                transform,
                color: na::Vector4::new(0.25, 0.25, 0.25, alpha),
            })
        }

        cur = next;
    }
}

pub fn render_machine<'a>(
    machine: &'a Machine,
    tick_time: &TickTime,
    exec: Option<&Exec>,
    filter: impl Fn(&'a grid::Point3) -> bool,
    unfocus: impl Fn(&'a grid::Point3) -> bool,
    out: &mut Stage,
) {
    out.floor.add(floor::Instance {
        size: na::Vector2::new(machine.size().x as f32, machine.size().y as f32),
    });

    for (block_index, (block_pos, placed_block)) in machine.iter_blocks() {
        if !filter(&block_pos) {
            continue;
        }

        let transform = placed_block_transform(&placed_block);
        let center = block_center(&block_pos);

        let anim_state = exec.map(|exec| AnimState::from_exec_block(exec, block_index));
        let level_progress = exec.and_then(|exec| exec.level_progress());
        let next_level_progress = exec.and_then(|exec| exec.next_level_progress());

        let alpha = if unfocus(&block_pos) {
            out.dither = true;
            0.55
        } else {
            1.0
        };

        render_block(
            &placed_block,
            tick_time,
            anim_state.as_ref(),
            level_progress,
            next_level_progress,
            &center,
            &transform,
            alpha,
            out,
        );

        if !placed_block.block.is_air() && !is_straight_pipe(&placed_block.block) {
            render_pillar(machine, block_pos, alpha, out);
        }

        out.dither = false;
    }
}

fn is_straight_pipe(block: &Block) -> bool {
    match block {
        Block::GeneralPipe(dirs) => grid::is_straight(dirs),
        _ => false,
    }
}
