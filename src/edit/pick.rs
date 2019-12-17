use std::iter;

use nalgebra as na;

use rendology::Camera;

use crate::machine::{grid, Machine};
use crate::render;
use crate::util::intersection::{ray_aabb_intersection, ray_plane_intersection, Plane, Ray, AABB};

pub fn camera_ray(camera: &Camera, eye: &na::Point3<f32>, window_pos: &na::Point2<f32>) -> Ray {
    let p_near = camera.unproject_from_viewport(&na::Point3::new(window_pos.x, window_pos.y, -1.0));
    let p_far = camera.unproject_from_viewport(&na::Point3::new(window_pos.x, window_pos.y, 1.0));

    Ray {
        origin: *eye,
        velocity: p_far - p_near,
    }
}

pub fn pick_in_layer_plane(
    machine: &Machine,
    layer: isize,
    camera: &Camera,
    eye: &na::Point3<f32>,
    window_pos: &na::Point2<f32>,
) -> Option<grid::Point3> {
    let ray = camera_ray(camera, eye, window_pos);
    let quad = Plane {
        origin: na::Point3::new(0.0, 0.0, layer as f32),
        direction_a: machine.size().x as f32 * na::Vector3::x(),
        direction_b: machine.size().y as f32 * na::Vector3::y(),
    };

    if let Some((ray_t, _plane_pos)) = ray_plane_intersection(&ray, &quad) {
        let ray_pos = ray.origin + ray_t * ray.velocity;
        let grid_pos = grid::Point3::new(
            ray_pos.x.floor() as isize,
            ray_pos.y.floor() as isize,
            layer,
        );

        // Intersection -- possibly at position outside of the grid though!
        Some(grid_pos)
    } else {
        // No intersection
        None
    }
}

pub fn pick_block(
    machine: &Machine,
    camera: &Camera,
    eye: &na::Point3<f32>,
    window_pos: &na::Point2<f32>,
) -> Option<grid::Point3> {
    let ray = camera_ray(camera, eye, window_pos);

    let mut closest_block = None;
    for (_block_index, (block_pos, _placed_block)) in machine.iter_blocks() {
        let center = render::machine::block_center(&block_pos);

        let aabb = AABB {
            min: center - na::Vector3::new(0.5, 0.5, 0.5),
            max: center + na::Vector3::new(0.5, 0.5, 0.5),
        };

        if let Some(distance) = ray_aabb_intersection(&ray, &aabb) {
            // TODO: Perform a tighter intersection check if AABB is a hit
            closest_block = Some(closest_block.map_or(
                (block_pos, distance),
                |(closest_pos, closest_distance)| {
                    if distance < closest_distance {
                        (block_pos, distance)
                    } else {
                        (closest_pos, closest_distance)
                    }
                },
            ));
        }
    }

    closest_block.map(|(pos, _distance)| *pos)
}

pub fn pick_line(machine: &Machine, a: &grid::Point3, b: &grid::Point3) -> Vec<grid::Point3> {
    let range = |a: isize, b: isize| {
        if a < b {
            (a..b).collect::<Vec<_>>()
        } else {
            (b..a).rev().collect::<Vec<_>>()
        }
    };

    // Move dimension by dimension from a to b.
    // (I think this doesn't fully make sense, but it's good enough for now.)

    let x = range(a.x, b.x)
        .into_iter()
        .map(|x| grid::Point3::new(x, a.y, a.z));
    let y = range(a.y, b.y)
        .into_iter()
        .map(|y| grid::Point3::new(b.x, y, a.z));
    let z = range(a.z, b.z)
        .into_iter()
        .map(|z| grid::Point3::new(b.x, b.y, z));

    let candidates = iter::once(*a)
        .chain(x)
        .chain(y)
        .chain(z)
        .chain(iter::once(*b));

    let mut points = Vec::new();

    for c in candidates {
        // We remove duplicates in a simple and costly way here that allows us
        // to keep the order (if b is included it should always be the last
        // element). We expect `points` to be relatively small anyway.
        if machine.is_block_at(&c) && !points.contains(&c) {
            points.push(c);
        }
    }

    points
}

pub fn pick_window_rect<'a>(
    machine: &'a Machine,
    camera: &'a Camera,
    window_a: &'a na::Point2<f32>,
    window_b: &'a na::Point2<f32>,
) -> impl Iterator<Item = grid::Point3> + 'a {
    let min = na::Point2::new(window_a.x.min(window_b.x), window_a.y.min(window_b.y));
    let max = na::Point2::new(window_a.x.max(window_b.x), window_a.y.max(window_b.y));

    machine
        .iter_blocks()
        .map(|(_block_index, (block_pos, _placed_block))| *block_pos)
        .filter(move |block_pos| {
            let center = render::machine::block_center(block_pos);
            let viewport_pos = camera.project_to_viewport(&center);

            viewport_pos.x >= min.x
                && viewport_pos.x <= max.x
                && viewport_pos.y >= min.y
                && viewport_pos.y <= max.y
        })
}
