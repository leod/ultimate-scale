use nalgebra as na;

use crate::machine::grid;

use crate::render::{Object, InstanceParams, RenderList};

pub struct Line {
    pub start: na::Point3<f32>,
    pub end: na::Point3<f32>,
    pub thickness: f32,
    pub color: na::Vector4<f32>,
}

pub fn render_line(line: &Line, out: &mut RenderList) {
    let center = line.start + (line.end - line.start) / 2.0;

    let look_at = na::Isometry3::face_towards(
        &center,
        &line.end,
        &na::Vector3::z(), // TODO
    );

    let scaling = na::Vector3::new(
        line.thickness,
        line.thickness,
        (line.end - line.start).norm(),
    );

    let transform = look_at.to_homogeneous() * na::Matrix4::new_nonuniform_scaling(&scaling);

    out.add(Object::Cube, &InstanceParams {
        transform,
        color: line.color,
        .. Default::default()
    });
}

pub fn render_xy_grid(size: &grid::Vector3, z: f32, out: &mut RenderList) {
    let thickness = 0.01;
    let color = na::Vector4::new(0.7, 0.7, 0.7, 1.0);

    for x in 0 .. size.x + 1 {
        render_line(
            &Line {
                start: na::Point3::new(x as f32, 0.0, z),
                end: na::Point3::new(x as f32, size.y as f32, z),
                thickness,
                color,
            },
            out,
        );
    }

    for y in 0 .. size.y + 1 {
        render_line(
            &Line {
                start: na::Point3::new(0.0, y as f32, z),
                end: na::Point3::new(size.x as f32, y as f32, z),
                thickness,
                color,
            },
            out,
        );
    }
}
