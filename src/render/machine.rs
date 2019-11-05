use nalgebra as na;

use crate::machine::grid::{self, Dir2, Dir3};
use crate::machine::{BlipKind, Block, Machine, PlacedBlock};

use crate::render::pipeline::{DefaultInstanceParams, RenderList, RenderLists};
use crate::render::Object;

use crate::exec::anim::{WindAnimState, WindLife};
use crate::exec::{Exec, TickTime};

pub const PIPE_THICKNESS: f32 = 0.05;

pub fn wind_source_color() -> na::Vector3<f32> {
    na::Vector3::new(1.0, 0.08, 0.24)
}

pub fn blip_spawn_color() -> na::Vector3<f32> {
    na::Vector3::new(60.0, 179.0, 113.0) / 255.0
}

pub fn blip_color(kind: BlipKind) -> na::Vector3<f32> {
    match kind {
        BlipKind::A => na::Vector3::new(0.0, 0.737, 0.361),
        BlipKind::B => na::Vector3::new(1.0, 0.557, 0.0),
        BlipKind::C => na::Vector3::new(0.098, 0.129, 0.694),
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub start: na::Point3<f32>,
    pub end: na::Point3<f32>,
    pub roll: f32,
    pub thickness: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_line(line: &Line, out: &mut RenderList<DefaultInstanceParams>) {
    let center = line.start + (line.end - line.start) / 2.0;

    let d = line.end - line.start;

    // This will roll the line somewhat, works nicely for the cuboid wireframe
    let up = d.cross(&na::Vector3::x()) + d.cross(&na::Vector3::y()) + d.cross(&na::Vector3::z());
    let rot = na::Rotation3::new(d.normalize() * line.roll);
    let look_at = na::Isometry3::face_towards(&center, &line.end, &(rot * up));

    let scaling = na::Vector3::new(
        line.thickness,
        line.thickness,
        (line.end - line.start).norm(),
    );

    let transform = look_at.to_homogeneous() * na::Matrix4::new_nonuniform_scaling(&scaling);

    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform,
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

impl Cuboid {
    pub fn corner_pos(&self, idx: [isize; 3]) -> na::Point3<f32> {
        let point: na::Point3<f32> = na::convert(na::Point3::from_slice(&idx));

        self.center + 0.5 * point.coords.component_mul(&self.size)
    }
}

#[rustfmt::skip]
pub fn render_cuboid_wireframe(
    cuboid: &Cuboid,
    thickness: f32,
    color: &na::Vector4<f32>,
    out: &mut RenderList<DefaultInstanceParams>,
) {
    let lines = vec![
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

    for (start_idx, end_idx) in lines {
        render_line(
            &Line {
                start: cuboid.corner_pos(start_idx),
                end: cuboid.corner_pos(end_idx),
                roll: 0.0,
                thickness,
                color: *color,
            },
            out
        );
    }
}

pub fn render_xy_grid(size: &grid::Vector3, z: f32, out: &mut RenderList<DefaultInstanceParams>) {
    let color = na::Vector4::new(0.7, 0.7, 0.7, 1.0);

    for x in 0..=size.x {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(x as f32, 0.0, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::y() * (size.y as f32)));
        out.add(
            Object::LineY,
            &DefaultInstanceParams {
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
            &DefaultInstanceParams {
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

pub fn render_bridge(
    dir: Dir2,
    length: f32,
    size: f32,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: &na::Vector4<f32>,
    out: &mut RenderList<DefaultInstanceParams>,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(dir.embed().to_vector());
    let output_transform = translation
        * transform
        * na::Matrix4::new_translation(&(dir_offset * length * 0.5))
        * na::Matrix4::new_rotation(dir.to_radians() * na::Vector3::z())
        * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(length, size, size));
    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform: output_transform,
            color: *color,
            ..Default::default()
        },
    );
}

pub fn render_mill(
    dir: Dir3,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    roll: f32,
    color: &na::Vector4<f32>,
    out: &mut RenderList<DefaultInstanceParams>,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let dir_offset: na::Vector3<f32> = na::convert(dir.to_vector());
    let (pitch, yaw) = dir.to_pitch_yaw_x();
    let length = 0.45;
    let transform = translation
        * transform
        * na::Matrix4::new_translation(&(dir_offset * length * 0.5))
        * na::Matrix4::from_euler_angles(roll, pitch, yaw)
        * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(length, 0.2, 0.07));
    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform,
            color: *color,
            ..Default::default()
        },
    );
}

pub fn render_wind_mills(
    placed_block: &PlacedBlock,
    tick_time: &TickTime,
    wind_anim_state: &Option<WindAnimState>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: &na::Vector4<f32>,
    out: &mut RenderLists,
) {
    for &dir in &Dir3::ALL {
        if !placed_block.has_wind_hole_out(dir) {
            continue;
        }

        let roll = wind_anim_state.as_ref().map_or(0.0, |anim| {
            // We need to apply the rotation because here we render
            // the blocks as if they were not rotated yet.
            // (Any rotation is contained in `transform`).
            let original_dir = placed_block.rotated_dir_xy(dir);

            let t = tick_time.tick_progress();

            std::f32::consts::PI
                * 2.0
                * match anim.wind_out(original_dir) {
                    WindLife::None => 0.0,
                    WindLife::Appearing => {
                        // The wind will start moving inside of the block, so
                        // delay mill rotation until the wind reaches the
                        // outside.
                        if t >= 0.5 {
                            t - 0.5
                        } else {
                            0.0
                        }
                    }
                    WindLife::Existing => t,
                    WindLife::Disappearing => {
                        // Stop mill rotation when wind reaches the inside of
                        // the block.
                        if t < 0.5 {
                            t
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
                &mut out.solid,
            );
        }
    }
}

pub fn render_pipe_bend(
    tick_time: &TickTime,
    wind_anim_state: &Option<WindAnimState>,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: &na::Vector4<f32>,
    out: &mut RenderList<DefaultInstanceParams>,
) {
    let translation = na::Matrix4::new_translation(&center.coords);
    let scaling =
        na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(PIPE_THICKNESS, 0.5, PIPE_THICKNESS));
    let offset = na::Matrix4::new_translation(&na::Vector3::new(0.0, -0.25, 0.0));

    // One rail
    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform: translation * transform * offset * scaling,
            color: *color,
            ..Default::default()
        },
    );

    // Another rail
    let rotation = na::Matrix4::new_rotation(na::Vector3::z() * std::f32::consts::PI / 2.0);
    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform: translation * transform * rotation * offset * scaling,
            color: *color,
            ..Default::default()
        },
    );

    // Pulsator to hide our shame of twist
    let size = 3.0
        * PIPE_THICKNESS
        * if let Some(wind_anim_state) = wind_anim_state.as_ref() {
            if wind_anim_state.num_alive_in() > 0 && wind_anim_state.num_alive_out() > 0 {
                1.0 + 0.1
                    * (tick_time.tick_progress() * 2.0 * std::f32::consts::PI)
                        .sin()
                        .powf(2.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

    let scaling = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(size, size, size));
    out.add(
        Object::Cube,
        &DefaultInstanceParams {
            transform: translation * transform * scaling,
            color: *color,
            ..Default::default()
        },
    );
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
        Block::PipeXY => {
            let scaling = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                PIPE_THICKNESS,
                1.0,
                PIPE_THICKNESS,
            ));

            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: translation * transform * scaling,
                    color: na::Vector4::new(0.75, 0.75, 0.75, alpha),
                    ..Default::default()
                },
            );
        }
        Block::PipeBendXY => render_pipe_bend(
            tick_time,
            wind_anim_state,
            center,
            transform,
            &na::Vector4::new(0.75, 0.75, 0.75, alpha),
            &mut out.solid,
        ),
        Block::PipeZ => {
            let rotation = na::Matrix4::new_rotation(na::Vector3::x() * std::f32::consts::PI / 2.0);
            out.solid.add(
                Object::PipeSegment,
                &DefaultInstanceParams {
                    transform: translation * transform * rotation,
                    color: na::Vector4::new(0.75, 0.75, 0.75, alpha),
                    ..Default::default()
                },
            );
        }
        Block::PipeBendZ { sign_z } => {
            let angle_y = -sign_z.to_f32() * std::f32::consts::PI / 2.0;
            let rotation = na::Matrix4::new_rotation(na::Vector3::y() * angle_y);

            render_pipe_bend(
                tick_time,
                wind_anim_state,
                center,
                &(transform * rotation),
                &na::Vector4::new(0.75, 0.75, 0.75, alpha),
                &mut out.solid,
            );
        }
        Block::PipeSplitXY { .. } => {
            out.solid.add(
                Object::PipeSplit,
                &DefaultInstanceParams {
                    transform: translation * transform,
                    color: na::Vector4::new(0.75, 0.75, 0.75, alpha),
                    ..Default::default()
                },
            );
        }
        Block::FunnelXY => {
            let cube_color = na::Vector4::new(1.0, 0.5, 0.5, alpha);
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.1, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.7, 0.8, 0.7));
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: cube_transform,
                    color: cube_color,
                    ..Default::default()
                },
            );

            let input_dir = Dir2::Y_NEG;
            let input_size = 0.6;

            let input_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, -0.3, 0.0))
                * na::Matrix4::new_rotation(input_dir.to_radians() * na::Vector3::z())
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                    0.9, input_size, input_size,
                ));
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: input_transform,
                    color: na::Vector4::new(1.0, 1.0, 1.0, alpha),
                    ..Default::default()
                },
            );
        }
        Block::WindSource => {
            let scaling = na::Matrix4::new_scaling(0.75);
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: translation * transform * scaling,
                    color: block_color(&wind_source_color(), alpha),
                    ..Default::default()
                },
            );

            render_wind_mills(
                placed_block,
                tick_time,
                wind_anim_state,
                center,
                transform,
                &na::Vector4::new(1.0, 1.0, 1.0, alpha),
                out,
            );
        }
        Block::BlipSpawn {
            kind,
            num_spawns,
            activated,
        } => {
            let cube_color = if num_spawns.is_some() {
                na::Vector4::new(0.0, 0.5, 0.0, alpha)
            } else {
                block_color(&blip_color(kind), alpha)
            };
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(-0.35 / 2.0, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.65, 1.0, 1.0));
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: cube_transform,
                    color: cube_color,
                    ..Default::default()
                },
            );

            let bridge_size = if num_spawns.is_some() { 0.15 } else { 0.3 };
            let bridge_length =
                bridge_length_animation(0.15, 0.75, activated.is_some(), tick_time.tick_progress());

            render_bridge(
                Dir2::X_POS,
                bridge_length,
                bridge_size,
                center,
                transform,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut out.solid,
            );
        }
        Block::BlipDuplicator {
            kind, activated, ..
        } => {
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.65, 1.0, 1.0));

            let kind_color = match activated.or(kind) {
                Some(kind) => blip_color(kind),
                None => na::Vector3::new(0.6, 0.6, 0.6),
            };
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: cube_transform,
                    color: block_color(&kind_color, alpha),
                    ..Default::default()
                },
            );

            let bridge_length =
                bridge_length_animation(0.35, 0.75, activated.is_some(), tick_time.tick_progress());

            render_bridge(
                Dir2::X_NEG,
                bridge_length,
                0.3,
                center,
                transform,
                &na::Vector4::new(0.8, 0.8, 0.8, alpha),
                &mut out.solid,
            );
            render_bridge(
                Dir2::X_POS,
                bridge_length,
                0.3,
                center,
                transform,
                &na::Vector4::new(0.8, 0.8, 0.8, alpha),
                &mut out.solid,
            );
        }
        Block::BlipWindSource { activated } => {
            let cube_color = if activated {
                block_color(&wind_source_color(), alpha)
            } else {
                na::Vector4::new(0.5, 0.0, 0.0, alpha)
            };

            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.75, 0.75, 0.75));
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: cube_transform,
                    color: cube_color,
                    ..Default::default()
                },
            );

            let button_length = if activated { 0.4 } else { 0.45 };
            render_bridge(
                Dir2::Y_NEG,
                button_length,
                0.5,
                center,
                transform,
                &na::Vector4::new(0.8, 0.8, 0.8, alpha),
                &mut out.solid,
            );

            render_wind_mills(
                placed_block,
                tick_time,
                wind_anim_state,
                center,
                transform,
                &na::Vector4::new(1.0, 1.0, 1.0, alpha),
                out,
            );
        }
        Block::Solid => {
            out.solid.add(
                Object::Cube,
                &DefaultInstanceParams {
                    transform: translation * transform,
                    color: na::Vector4::new(0.3, 0.2, 0.9, alpha),
                    ..Default::default()
                },
            );
        }
    }
}

pub fn render_arrow(line: &Line, _roll: f32, out: &mut RenderList<DefaultInstanceParams>) {
    render_line(line, out);

    // TODO
    /*let head_transform = na::Matrix4::face_towards(
        &line.end,
        &(line.end + (line.end - line.start)),
        &na::Vector3::y(),
    ) * na::Matrix4::new_scaling(0.9);

    out.add(
        Object::Triangle,
        &DefaultInstanceParams {
            transform: head_transform,
            color: line.color,
            ..Default::default()
        },
    );*/
}

pub fn block_center(pos: &grid::Point3) -> na::Point3<f32> {
    let coords_float: na::Vector3<f32> = na::convert(pos.coords);
    na::Point3::from(coords_float) + na::Vector3::new(0.5, 0.5, 0.5)
}

pub fn placed_block_transform(placed_block: &PlacedBlock) -> na::Matrix4<f32> {
    na::Matrix4::new_rotation(placed_block.angle_xy_radians() * na::Vector3::z())
}

pub fn render_machine(
    machine: &Machine,
    tick_time: &TickTime,
    exec: Option<&Exec>,
    out: &mut RenderLists,
) {
    let floor_size = na::Vector3::new(machine.size().x as f32, machine.size().y as f32, 1.0);

    let floor_transform = na::Matrix4::new_nonuniform_scaling(&floor_size);
    out.solid.add(
        Object::Quad,
        &DefaultInstanceParams {
            transform: floor_transform,
            //color: na::Vector4::new(0.1608, 0.4235, 0.5725, 1.0),
            color: na::Vector4::new(0.33, 0.64, 0.82, 1.0),
            ..Default::default()
        },
    );

    for (block_index, (block_pos, placed_block)) in machine.iter_blocks() {
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
