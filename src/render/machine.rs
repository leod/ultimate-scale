use nalgebra as na;

use crate::machine::grid::{self, Dir2};
use crate::machine::{Block, Machine, PlacedBlock};

use crate::render::{InstanceParams, Object, RenderList, RenderLists};

#[derive(Clone, Debug)]
pub struct Line {
    pub start: na::Point3<f32>,
    pub end: na::Point3<f32>,
    pub thickness: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_line(line: &Line, out: &mut RenderList) {
    let center = line.start + (line.end - line.start) / 2.0;

    let d = line.end - line.start;

    // This will roll the line somewhat, works nicely for the cuboid wireframe
    let up = d.cross(&na::Vector3::x()) + d.cross(&na::Vector3::y()) + d.cross(&na::Vector3::z());

    let look_at = na::Isometry3::face_towards(&center, &line.end, &up);

    let scaling = na::Vector3::new(
        line.thickness,
        line.thickness,
        (line.end - line.start).norm(),
    );

    let transform = look_at.to_homogeneous() * na::Matrix4::new_nonuniform_scaling(&scaling);

    out.add(
        Object::Cube,
        &InstanceParams {
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
    out: &mut RenderList,
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
                thickness,
                color: *color,
            },
            out
        );
    }
}

pub fn render_xy_grid(size: &grid::Vector3, z: f32, out: &mut RenderList) {
    let color = na::Vector4::new(0.7, 0.7, 0.7, 1.0);

    for x in 0..=size.x {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(x as f32, 0.0, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::y() * (size.y as f32)));
        out.add(
            Object::LineY,
            &InstanceParams {
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
            &InstanceParams {
                transform: translation * scaling,
                color,
                ..Default::default()
            },
        );
    }
}

pub fn render_block(
    block: &Block,
    center: &na::Point3<f32>,
    transform: &na::Matrix4<f32>,
    color: Option<&na::Vector4<f32>>,
    alpha: f32,
    out: &mut RenderList,
) {
    let translation = na::Matrix4::new_translation(&center.coords);

    match block {
        Block::PipeXY => {
            out.add(
                Object::PipeSegment,
                &InstanceParams {
                    transform: translation * transform,
                    color: *color.unwrap_or(&na::Vector4::new(0.75, 0.75, 0.75, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::PipeBendXY => {
            let rotation = na::Matrix4::new_rotation(na::Vector3::z() * std::f32::consts::PI);
            out.add(
                Object::PipeBend,
                &InstanceParams {
                    transform: translation * transform * rotation,
                    color: *color.unwrap_or(&na::Vector4::new(0.75, 0.75, 0.75, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::PipeZ => {
            let rotation = na::Matrix4::new_rotation(na::Vector3::x() * std::f32::consts::PI / 2.0);
            out.add(
                Object::PipeSegment,
                &InstanceParams {
                    transform: translation * transform * rotation,
                    color: *color.unwrap_or(&na::Vector4::new(0.75, 0.75, 0.75, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::PipeBendZ { sign_z } => {
            let angle_y = -sign_z.to_f32() * std::f32::consts::PI / 2.0;
            let rotation = na::Matrix4::new_rotation(na::Vector3::y() * angle_y)
                * na::Matrix4::new_rotation(na::Vector3::z() * std::f32::consts::PI);
            out.add(
                Object::PipeBend,
                &InstanceParams {
                    transform: translation * transform * rotation,
                    color: *color.unwrap_or(&na::Vector4::new(0.75, 0.75, 0.75, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::PipeSplitXY { .. } => {
            out.add(
                Object::PipeSplit,
                &InstanceParams {
                    transform: translation * transform,
                    color: *color.unwrap_or(&na::Vector4::new(0.75, 0.75, 0.75, alpha)),
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
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: cube_transform,
                    color: *color.unwrap_or(&cube_color),
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
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: input_transform,
                    color: *color.unwrap_or(&na::Vector4::new(1.0, 1.0, 1.0, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::WindSource => {
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: translation * transform,
                    color: *color.unwrap_or(&na::Vector4::new(1.0, 0.0, 0.0, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::BlipSpawn { num_spawns, .. } => {
            let cube_color = if num_spawns.is_some() {
                na::Vector4::new(0.0, 0.5, 0.0, alpha)
            } else {
                na::Vector4::new(0.0, 1.0, 0.0, alpha)
            };
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(-0.35 / 2.0, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.65, 1.0, 1.0));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: cube_transform,
                    color: *color.unwrap_or(&cube_color),
                    ..Default::default()
                },
            );

            let output_dir = Dir2::X_POS;
            let output_size = if num_spawns.is_some() { 0.15 } else { 0.3 };

            let output_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.3, 0.0, 0.0))
                * na::Matrix4::new_rotation(output_dir.to_radians() * na::Vector3::z())
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                    1.0,
                    output_size,
                    output_size,
                ));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: output_transform,
                    color: *color.unwrap_or(&na::Vector4::new(1.0, 1.0, 1.0, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::BlipDuplicator { .. } => {
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(0.65, 1.0, 1.0));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: cube_transform,
                    color: *color.unwrap_or(&na::Vector4::new(0.0, 1.0, 0.0, alpha)),
                    ..Default::default()
                },
            );

            let output_dir = Dir2::X_POS;
            let output_size = 0.3;
            let output_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.3, 0.0, 0.0))
                * na::Matrix4::new_rotation(output_dir.to_radians() * na::Vector3::z())
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                    1.0,
                    output_size,
                    output_size,
                ));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: output_transform,
                    color: *color.unwrap_or(&na::Vector4::new(1.0, 1.0, 1.0, alpha)),
                    ..Default::default()
                },
            );

            let output_dir = Dir2::X_NEG;
            let output_size = 0.3;
            let output_transform = translation
                * transform
                * na::Matrix4::new_rotation(output_dir.to_radians() * na::Vector3::z())
                * na::Matrix4::new_translation(&na::Vector3::new(0.3, 0.0, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                    1.0,
                    output_size,
                    output_size,
                ));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: output_transform,
                    color: *color.unwrap_or(&na::Vector4::new(1.0, 1.0, 1.0, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::BlipWindSource { activated } => {
            let cube_color = if *activated {
                na::Vector4::new(0.5, 0.0, 0.0, alpha)
            } else {
                na::Vector4::new(1.0, 0.0, 0.0, alpha)
            };
            let cube_transform = translation
                * transform
                * na::Matrix4::new_translation(&na::Vector3::new(0.0, 0.1, 0.0))
                * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(1.0, 0.8, 1.0));
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: cube_transform,
                    color: *color.unwrap_or(&cube_color),
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
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: input_transform,
                    color: *color.unwrap_or(&na::Vector4::new(0.0, 1.0, 0.0, alpha)),
                    ..Default::default()
                },
            );
        }
        Block::Solid => {
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: translation * transform,
                    color: *color.unwrap_or(&na::Vector4::new(0.3, 0.9, 0.2, alpha)),
                    ..Default::default()
                },
            );
        }
    }
}

pub fn render_arrow(line: &Line, _roll: f32, out: &mut RenderList) {
    render_line(line, out);

    // TODO
    /*let head_transform = na::Matrix4::face_towards(
        &line.end,
        &(line.end + (line.end - line.start)),
        &na::Vector3::y(),
    ) * na::Matrix4::new_scaling(0.9);

    out.add(
        Object::Triangle,
        &InstanceParams {
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

pub fn render_machine(machine: &Machine, out: &mut RenderLists) {
    let floor_size = na::Vector3::new(machine.size().x as f32, machine.size().y as f32, 1.0);

    let floor_transform = na::Matrix4::new_nonuniform_scaling(&floor_size);
    out.solid.add(
        Object::Quad,
        &InstanceParams {
            transform: floor_transform,
            //color: na::Vector4::new(0.1608, 0.4235, 0.5725, 1.0),
            color: na::Vector4::new(0.33, 0.64, 0.82, 1.0),
            ..Default::default()
        },
    );

    for (_index, (block_pos, placed_block)) in machine.iter_blocks() {
        let transform = placed_block_transform(&placed_block);
        let center = block_center(&block_pos);

        render_block(
            &placed_block.block,
            &center,
            &transform,
            None,
            1.0,
            &mut out.solid_shadow,
        );
        render_block(
            &placed_block.block,
            &center,
            &transform,
            None,
            1.0,
            &mut out.solid,
        );
    }
}
