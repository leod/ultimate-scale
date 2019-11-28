use nalgebra as na;

use crate::machine::grid::{self, Dir3};
use crate::machine::{level, BlipKind, Block, Machine, PlacedBlock};

use crate::render::pipeline::scene::model;
use crate::render::pipeline::{Light, RenderLists};
use crate::render::{Object, RenderList};

use crate::exec::anim::{WindAnimState, WindLife};
use crate::exec::{Exec, TickTime};

pub const PIPE_THICKNESS: f32 = 0.05;
pub const MILL_THICKNESS: f32 = 0.2;
pub const MILL_DEPTH: f32 = 0.09;
pub const OUTLINE_THICKNESS: f32 = 0.02;
pub const OUTLINE_MARGIN: f32 = 0.0005;
pub const BRIDGE_MARGIN: f32 = 0.005;

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

pub fn blip_spawn_color() -> na::Vector3<f32> {
    gamma_correct(&(na::Vector3::new(60.0, 179.0, 113.0) / 255.0))
}

pub fn blip_color(kind: BlipKind) -> na::Vector3<f32> {
    gamma_correct(&match kind {
        BlipKind::A => na::Vector3::new(0.2, 0.2, 0.8),
        BlipKind::B => na::Vector3::new(0.0, 0.737, 0.361),
        BlipKind::C => na::Vector3::new(0.098, 0.129, 0.694),
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
    gamma_correct(&na::Vector3::new(0.3, 0.3, 0.3))
}

pub fn grid_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.578, 0.578, 0.578))
}

pub fn outline_color() -> na::Vector3<f32> {
    gamma_correct(&na::Vector3::new(0.01, 0.01, 0.01))
}

#[derive(Clone, Debug)]
pub struct Line {
    pub start: na::Point3<f32>,
    pub end: na::Point3<f32>,
    pub roll: f32,
    pub thickness: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_line(line: &Line, transform: &na::Matrix4<f32>, out: &mut RenderList<model::Params>) {
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

    out.add(
        Object::Cube,
        &model::Params {
            transform: cube_transform,
            color: line.color,
            ..Default::default()
        },
    );
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
    out: &mut RenderList<model::Params>,
) {
    for (start, end) in CUBOID_WIREFRAME_LINES.iter() {
        let start: na::Point3<f32> = na::convert(na::Point3::from_slice(start));
        let end: na::Point3<f32> = na::convert(na::Point3::from_slice(end));

        render_line(
            &Line {
                start: start / 2.0,
                end: end / 2.0,
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
    out: &mut RenderList<model::Params>,
) {
    let transform = na::Matrix4::new_translation(&cuboid.center.coords)
        * na::Matrix4::new_nonuniform_scaling(&cuboid.size);

    render_cuboid_wireframe_with_transform(thickness, color, &transform, out);
}

pub fn render_xy_grid(size: &grid::Vector3, z: f32, out: &mut RenderList<model::Params>) {
    let color = block_color(&grid_color(), 0.0);

    for x in 0..=size.x {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(x as f32, 0.0, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::y() * (size.y as f32)));
        out.add(
            Object::LineY,
            &model::Params {
                transform: translation * scaling,
                color,
                ..Default::default()
            },
        );
    }

    for y in 0..=size.y {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(0.0, y as f32, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::x() * (size.x as f32)));
        out.add(
            Object::LineX,
            &model::Params {
                transform: translation * scaling,
                color,
                ..Default::default()
            },
        );
    }
}

pub fn bridge_length_animation(min: f32, max: f32, activated: bool, progress: f32) -> f32 {
    min + (if activated && progress <= 1.0 {
        let x = progress * std::f32::consts::PI;
        x.cos().abs()
    } else {
        1.0
    }) * (max - min)
}

pub fn block_color(color: &na::Vector3<f32>, alpha: f32) -> na::Vector4<f32> {
    na::Vector4::new(color.x, color.y, color.z, alpha)
}

pub struct Bridge {
    pub center: na::Point3<f32>,
    pub dir: Dir3,
    pub offset: f32,
    pub length: f32,
    pub size: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_bridge(bridge: &Bridge, transform: &na::Matrix4<f32>, out: &mut RenderLists) {
    let translation = na::Matrix4::new_translation(&bridge.center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(bridge.dir.to_vector());
    let (pitch, yaw) = bridge.dir.to_pitch_yaw_x();
    let output_transform = translation
        * transform
        * na::Matrix4::new_translation(
            &(dir_offset * (0.5 * bridge.length + bridge.offset + BRIDGE_MARGIN)),
        )
        * na::Matrix4::from_euler_angles(0.0, pitch, yaw);
    let scaling = na::Vector3::new(bridge.length, bridge.size, bridge.size);
    out.solid.add(
        Object::Cube,
        &model::Params {
            transform: output_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
            color: bridge.color,
            ..Default::default()
        },
    );
    render_outline(&output_transform, &scaling, bridge.color.w, out);
}

pub fn render_mill(
    dir: Dir3,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    roll: f32,
    color: &na::Vector4<f32>,
    offset: f32,
    length: f32,
    out: &mut RenderLists,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(dir.to_vector());
    let (pitch, yaw) = dir.to_pitch_yaw_x();
    let cube_transform = translation
        * transform
        * na::Matrix4::new_translation(&(dir_offset * (length * 0.5 + offset + BRIDGE_MARGIN)))
        * na::Matrix4::from_euler_angles(roll, pitch, yaw);
    let scaling = na::Vector3::new(length, MILL_THICKNESS, MILL_DEPTH);
    out.solid.add(
        Object::Cube,
        &model::Params {
            transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
            color: *color,
            ..Default::default()
        },
    );
    //render_outline(&cube_transform, &scaling, color.w, out);
}

pub fn render_wind_mills(
    placed_block: &PlacedBlock,
    tick_time: &TickTime,
    wind_anim_state: &Option<WindAnimState>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: &na::Vector4<f32>,
    offset: f32,
    length: f32,
    out: &mut RenderLists,
) {
    for &dir in &Dir3::ALL {
        if !placed_block.has_wind_hole_out(dir) {
            continue;
        }

        let roll = wind_anim_state.as_ref().map_or(0.0, |anim| {
            let t = tick_time.tick_progress();

            if anim.out_deadend(dir).is_some() {
                return 0.0;
            }

            let wind_time_offset = offset + length;

            std::f32::consts::PI / 2.0
                * match anim.wind_out(dir) {
                    WindLife::None => 0.0,
                    WindLife::Appearing => {
                        // The wind will start moving inside of the block, so
                        // delay mill rotation until the wind reaches the
                        // outside.
                        if t >= wind_time_offset {
                            (t - wind_time_offset) / (1.0 - wind_time_offset)
                        } else {
                            0.0
                        }
                    }
                    WindLife::Existing => t,
                    WindLife::Disappearing => {
                        // Stop mill rotation when wind reaches the inside of
                        // the block.
                        if t < wind_time_offset {
                            t / wind_time_offset
                        } else {
                            0.0
                        }
                    }
                }
        });

        for &phase in &[0.0, 0.25] {
            render_mill(
                dir,
                center,
                transform,
                roll + 2.0 * phase * std::f32::consts::PI,
                color,
                offset,
                length,
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
    out: &mut RenderList<model::Params>,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let scaling =
        na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.5, PIPE_THICKNESS, PIPE_THICKNESS));
    let offset = na::Matrix4::new_translation(&na::Vector3::new(-0.25, 0.0, 0.0));

    let (pitch, yaw) = dir.invert().to_pitch_yaw_x();
    let rotation = na::Matrix4::from_euler_angles(0.0, pitch, yaw);

    out.add(
        Object::Cube,
        &model::Params {
            transform: translation * transform * rotation * offset * scaling,
            color: *color,
            ..Default::default()
        },
    );
}

pub fn render_outline(
    cube_transform: &na::Matrix4<f32>,
    scaling: &na::Vector3<f32>,
    alpha: f32,
    out: &mut RenderLists,
) {
    let transform = cube_transform
        * na::Matrix4::new_nonuniform_scaling(
            &(scaling + na::Vector3::new(OUTLINE_MARGIN, OUTLINE_MARGIN, OUTLINE_MARGIN)),
        );

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

        out.plain.add(
            Object::LineX,
            &model::Params {
                transform: line_transform,
                color: block_color(&outline_color(), alpha),
                ..Default::default()
            },
        );
    }
}

pub fn render_block(
    placed_block: &PlacedBlock,
    tick_time: &TickTime,
    wind_anim_state: &Option<WindAnimState>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    alpha: f32,
    out: &mut RenderLists,
) {
    let translation = na::Matrix4::new_translation(&center.coords);

    match placed_block.block {
        Block::Pipe(dir_a, dir_b) => {
            let color = block_color(&pipe_color(), alpha);

            render_half_pipe(center, transform, dir_a, &color, &mut out.solid);
            render_half_pipe(center, transform, dir_b, &color, &mut out.solid);

            // Pulsator to hide our shame of wind direction change
            if dir_a.0 != dir_b.0 {
                let size = 2.5
                    * PIPE_THICKNESS
                    * if let Some(wind_anim_state) = wind_anim_state.as_ref() {
                        if wind_anim_state.num_alive_in() > 0 && wind_anim_state.num_alive_out() > 0
                        {
                            1.0 + 0.05
                                * (tick_time.tick_progress() * std::f32::consts::PI)
                                    .sin()
                                    .powf(2.0)
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };

                let cube_transform = translation * transform;
                let scaling = na::Vector3::new(size, size, size);

                out.solid.add(
                    Object::Cube,
                    &model::Params {
                        transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                        color,
                        ..Default::default()
                    },
                );

                render_outline(&cube_transform, &scaling, alpha, out);
            }
        }
        Block::PipeMergeXY => {
            let scaling = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                PIPE_THICKNESS,
                1.0,
                PIPE_THICKNESS,
            ));

            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: translation * transform * scaling,
                    color: block_color(&pipe_color(), alpha),
                    ..Default::default()
                },
            );

            let transform = transform
                * na::Matrix4::new_rotation(na::Vector3::z() * std::f32::consts::PI / 2.0);
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: translation * transform * scaling,
                    color: block_color(&pipe_color(), alpha),
                    ..Default::default()
                },
            );
        }
        Block::FunnelXY { flow_dir } => {
            let (pitch, yaw) = flow_dir.invert().to_pitch_yaw_x();
            let cube_transform = translation
                * transform
                * na::Matrix4::from_euler_angles(0.0, pitch, yaw)
                * na::Matrix4::new_translation(&na::Vector3::new(0.1, 0.0, 0.0));
            let scaling = na::Vector3::new(0.8, 0.6, 0.6);

            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: block_color(&funnel_in_color(), alpha),
                    ..Default::default()
                },
            );
            render_outline(&cube_transform, &scaling, alpha, out);

            let input_size = 0.4;
            let input_transform = translation
                * transform
                * na::Matrix4::from_euler_angles(0.0, pitch, yaw)
                * na::Matrix4::new_translation(&na::Vector3::new(-0.3, 0.0, 0.0));
            let scaling = &na::Vector3::new(0.9, input_size, input_size);
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: input_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: block_color(&funnel_out_color(), alpha),
                    ..Default::default()
                },
            );
            render_outline(&input_transform, &scaling, alpha, out);
        }
        Block::WindSource => {
            let cube_transform = translation * transform;
            let scaling = na::Vector3::new(0.6, 0.6, 0.6);

            let render_list = if wind_anim_state.is_some() {
                &mut out.solid_glow
            } else {
                &mut out.solid
            };
            render_list.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: block_color(&wind_source_color(), alpha),
                    ..Default::default()
                },
            );

            render_outline(&cube_transform, &scaling, alpha, out);

            if wind_anim_state.is_some() {
                out.lights.push(Light {
                    position: *center,
                    attenuation: na::Vector3::new(1.0, 0.0, 3.0),
                    color: 8.0 * wind_source_color(),
                    ..Default::default()
                });
            }

            render_wind_mills(
                placed_block,
                tick_time,
                wind_anim_state,
                center,
                transform,
                &na::Vector4::new(1.0, 1.0, 1.0, alpha),
                0.3,
                0.075,
                out,
            );
        }
        Block::BlipSpawn {
            out_dir,
            kind,
            num_spawns,
            activated,
        } => {
            let cube_color = block_color(&blip_color(kind), alpha);
            let (pitch, yaw) = out_dir.to_pitch_yaw_x();
            let cube_transform = translation
                * transform
                * na::Matrix4::from_euler_angles(0.0, pitch, yaw)
                * na::Matrix4::new_translation(&na::Vector3::new(-0.35 / 2.0, 0.0, 0.0));
            let scaling = na::Vector3::new(0.65, 0.95, 0.95);
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: cube_color,
                    ..Default::default()
                },
            );
            render_outline(&cube_transform, &scaling, alpha, out);

            let bridge_size = if num_spawns.is_some() { 0.15 } else { 0.3 };
            let bridge_length =
                bridge_length_animation(0.05, 0.6, activated.is_some(), tick_time.tick_progress());

            render_bridge(
                &Bridge {
                    center: *center,
                    dir: out_dir,
                    offset: 0.65 / 2.0 - 0.35 / 2.0,
                    length: bridge_length,
                    size: bridge_size,
                    color: block_color(&patient_bridge_color(), alpha),
                },
                transform,
                out,
            );
        }
        Block::BlipDuplicator {
            out_dirs,
            kind,
            activated,
            ..
        } => {
            let (pitch, yaw) = out_dirs.0.to_pitch_yaw_x();
            let cube_transform =
                translation * transform * na::Matrix4::from_euler_angles(0.0, pitch, yaw);
            let scaling = na::Vector3::new(0.65, 0.95, 0.95);

            let kind_color = match activated.or(kind) {
                Some(kind) => blip_color(kind),
                None => inactive_blip_duplicator_color(),
            };
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: block_color(&kind_color, alpha),
                    ..Default::default()
                },
            );
            render_outline(&cube_transform, &scaling, alpha, out);

            let bridge_length =
                bridge_length_animation(0.05, 0.4, activated.is_some(), tick_time.tick_progress());

            for &dir in &[out_dirs.0, out_dirs.1] {
                render_bridge(
                    &Bridge {
                        center: *center,
                        dir,
                        offset: 0.65 / 2.0,
                        length: bridge_length,
                        size: 0.3,
                        color: block_color(&impatient_bridge_color(), alpha),
                    },
                    transform,
                    out,
                );
            }
        }
        Block::BlipWindSource {
            button_dir,
            activated,
        } => {
            let cube_color = block_color(
                &if activated {
                    wind_source_color()
                } else {
                    inactive_blip_wind_source_color()
                },
                alpha,
            );

            let render_list = if activated {
                &mut out.solid_glow
            } else {
                &mut out.solid
            };

            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0));
            let scaling = na::Vector3::new(0.75, 0.75, 0.75);
            render_list.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color: cube_color,
                    ..Default::default()
                },
            );
            render_outline(&cube_transform, &scaling, alpha, out);

            if activated {
                out.lights.push(Light {
                    position: *center,
                    attenuation: na::Vector3::new(1.0, 0.0, 3.0),
                    color: 8.0 * wind_source_color(),
                    ..Default::default()
                });
            }

            let button_length = if activated { 0.025 } else { 0.075 };
            render_bridge(
                &Bridge {
                    center: *center,
                    dir: button_dir,
                    offset: 0.75 / 2.0,
                    length: button_length,
                    size: 0.5,
                    color: block_color(&impatient_bridge_color(), alpha),
                },
                transform,
                out,
            );

            render_wind_mills(
                placed_block,
                tick_time,
                wind_anim_state,
                center,
                transform,
                &block_color(&wind_mill_color(), alpha),
                0.75 / 2.0,
                0.075,
                out,
            );
        }
        Block::Solid => {
            let cube_transform = translation * transform;
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform,
                    color: block_color(&solid_color(), alpha),
                    ..Default::default()
                },
            );
            render_outline(
                &cube_transform,
                &na::Vector3::new(1.0, 1.0, 1.0),
                alpha,
                out,
            );
        }
        Block::Input {
            out_dir, activated, ..
        } => {
            let is_wind_active = wind_anim_state
                .as_ref()
                .map_or(false, |anim| anim.wind_out(Dir3::X_POS).is_alive());
            let active_blip_kind = match activated {
                None => None,
                Some(level::Input::Blip(kind)) => Some(kind),
            };

            let angle = std::f32::consts::PI / 4.0
                + if is_wind_active {
                    tick_time.tick_progress() * std::f32::consts::PI
                } else {
                    0.0
                };
            let rotation = na::Matrix4::from_euler_angles(angle, 0.0, 0.0);

            let color = block_color(
                &active_blip_kind.map_or(na::Vector3::new(0.3, 0.3, 0.3), blip_color),
                alpha,
            );

            let cube_transform = translation * transform * rotation;
            let scaling = na::Vector3::new(0.8, 0.6, 0.6);
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: cube_transform * na::Matrix4::new_nonuniform_scaling(&scaling),
                    color,
                    ..Default::default()
                },
            );
            render_outline(&cube_transform, &scaling, alpha, out);

            let bridge_length = bridge_length_animation(
                0.05,
                0.35,
                active_blip_kind.is_some(),
                tick_time.tick_progress(),
            );

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
        Block::Output {
            in_dir,
            ref outputs,
            failed,
            activated,
            ..
        } => {
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

            // Foolish stuff to transition to the next expected color mid-tick
            let transition_time = 0.6;
            let expected_kind =
                if activated.is_none() || tick_time.tick_progress() < transition_time {
                    outputs.last().copied()
                } else {
                    if outputs.len() > 1 {
                        outputs.get(outputs.len() - 2).copied()
                    } else {
                        None
                    }
                };

            let completed = (tick_time.tick_progress() >= 0.45
                && outputs.len() == 1
                && activated == outputs.last().copied())
                || (outputs.is_empty() && wind_anim_state.is_some());

            let status_color = output_status_color(failed, completed);
            let floor_translation = na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, -0.5));
            let floor_scaling =
                na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.8, 0.8, 0.15));
            out.solid.add(
                Object::Cube,
                &model::Params {
                    transform: translation * floor_translation * transform * floor_scaling,
                    color: block_color(&status_color, alpha),
                    ..Default::default()
                },
            );

            let expected_next_color = block_color(
                &expected_kind.map_or(impatient_bridge_color(), blip_color),
                alpha,
            );

            let thingy_translation =
                na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, -0.3));
            let thingy_scaling =
                na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.2, 0.2, 0.4));
            out.solid_glow.add(
                Object::Cube,
                &model::Params {
                    transform: translation * thingy_translation * transform * thingy_scaling,
                    color: expected_next_color,
                    ..Default::default()
                },
            );
        }
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

pub fn render_machine<'a>(
    machine: &'a Machine,
    tick_time: &TickTime,
    exec: Option<&Exec>,
    filter: impl Fn(&'a grid::Point3) -> bool,
    out: &mut RenderLists,
) {
    let floor_size = na::Vector3::new(machine.size().x as f32, machine.size().y as f32, 1.0);

    let floor_transform = na::Matrix4::new_nonuniform_scaling(&floor_size);
    out.solid.add(
        Object::Quad,
        &model::Params {
            transform: floor_transform,
            color: block_color(&floor_color(), 1.0),
            ..Default::default()
        },
    );

    for (block_index, (block_pos, placed_block)) in machine.iter_blocks() {
        if !filter(&block_pos) {
            continue;
        }

        let transform = placed_block_transform(&placed_block);
        let center = block_center(&block_pos);

        let wind_anim_state = exec.map(|exec| WindAnimState::from_exec_block(exec, block_index));

        render_block(
            &placed_block,
            tick_time,
            &wind_anim_state,
            &center,
            &transform,
            1.0,
            out,
        );
    }
}
