use nalgebra as na;

use crate::machine::grid;
use crate::machine::{Machine, Block};

use crate::render::{Object, InstanceParams, RenderList};

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

pub fn render_block(block: &Block, transform: &na::Matrix4<f32>, out: &mut RenderList) {
    match block {
        Block::Solid => {
            out.add(
                Object::PipeSegment,
                &InstanceParams {
                    transform: transform.clone(),
                    color: na::Vector4::new(0.6, 0.6, 0.6, 1.0),
                    .. Default::default()
                },
            );
        }
        _ => (),
    }
}

pub fn render_machine(machine: &Machine, out: &mut RenderList) {
    let half_vec = na::Vector3::new(0.5, 0.5, 0.5);

    for (block_pos, block) in machine.iter_blocks() {
        let block_coords: na::Vector3<f32> = na::convert(block_pos.coords);
        let transform = na::Matrix4::new_translation(&(block_coords + half_vec));

        render_block(block, &transform, out);
    }
}
