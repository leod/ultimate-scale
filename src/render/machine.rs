use nalgebra as na;

use crate::machine::grid;
use crate::machine::{Machine, Block, PlacedBlock};

use crate::render::{Object, InstanceParams, RenderList, RenderLists};

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

    let look_at = na::Isometry3::face_towards(
        &center,
        &line.end,
        &up,
    );

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
            .. Default::default()
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

    for x in 0 .. size.x + 1 {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(x as f32, 0.0, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::y() * (size.y as f32)));
        out.add(
            Object::LineY,
            &InstanceParams {
                transform: translation * scaling,
                color,
                .. Default::default()
            }, 
        );
    }

    for y in 0 .. size.y + 1 {
        let translation = na::Matrix4::new_translation(&na::Vector3::new(0.0, y as f32, z));
        let scaling = na::Matrix4::new_nonuniform_scaling(&(na::Vector3::x() * (size.x as f32)));
        out.add(
            Object::LineX,
            &InstanceParams {
                transform: translation * scaling,
                color,
                .. Default::default()
            }, 
        );
    }
}

pub fn render_block(
    block: &Block,
    transform: &na::Matrix4<f32>,
    color: Option<&na::Vector4<f32>>,
    out: &mut RenderList,
) {
    match block {
        Block::PipeXY => {
            out.add(
                Object::PipeSegment,
                &InstanceParams {
                    transform: transform.clone(),
                    color: *color.unwrap_or(&na::Vector4::new(0.6, 0.6, 0.6, 1.0)),
                    .. Default::default()
                },
            );
        }
        Block::PipeSplitXY => {
            out.add(
                Object::PipeSplit,
                &InstanceParams {
                    transform: transform.clone(),
                    color: *color.unwrap_or(&na::Vector4::new(0.6, 0.6, 0.6, 1.0)),
                    .. Default::default()
                },
            );
        }
        Block::PipeBendXY => {
            out.add(
                Object::PipeBend,
                &InstanceParams {
                    transform: transform.clone(),
                    color: *color.unwrap_or(&na::Vector4::new(0.6, 0.6, 0.6, 1.0)),
                    .. Default::default()
                },
            );
        }
        Block::Solid => {
            out.add(
                Object::Cube,
                &InstanceParams {
                    transform: transform.clone(),
                    color: *color.unwrap_or(&na::Vector4::new(0.3, 0.9, 0.2, 1.0)),
                    .. Default::default()
                },
            );
        }
    }
}

pub fn placed_block_transform(pos: &grid::Point3, dir_xy: &grid::Dir2) -> na::Matrix4<f32> {
    let angle_radians = dir_xy.to_radians();
    let rotation = na::Matrix4::new_rotation(angle_radians * na::Vector3::z());

    let coords: na::Vector3<f32> = na::convert(pos.coords);
    let translation = na::Matrix4::new_translation(&(coords + na::Vector3::new(0.5, 0.5, 0.5)));

    translation * rotation
}

pub fn render_machine(machine: &Machine, out: &mut RenderLists) {
    let floor_size = na::Vector3::new(
        machine.size().x as f32,
        machine.size().y as f32,
        1.0
    );

    let floor_transform = na::Matrix4::new_nonuniform_scaling(&floor_size);
    out.solid.add(
        Object::Quad,
        &InstanceParams {
            transform: floor_transform,
            //color: na::Vector4::new(0.1608, 0.4235, 0.5725, 1.0),
            color: na::Vector4::new(0.33, 0.64, 0.82, 1.0),
            .. Default::default()
        },
    );

    for (_index, (block_pos, placed_block)) in machine.iter_blocks() {
        let transform = placed_block_transform(&block_pos, &placed_block.dir_xy);

        render_block(&placed_block.block, &transform, None, &mut out.solid_shadow);
        render_block(&placed_block.block, &transform, None, &mut out.solid);
    }
}
