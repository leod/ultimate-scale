use nalgebra as na;

use crate::machine::{grid, Machine};
use crate::render::{self, Camera};
use crate::util::intersection::{ray_aabb_intersection, ray_quad_intersection, Plane, Ray, AABB};

pub fn camera_ray(camera: &Camera, eye: &na::Point3<f32>, window_pos: &na::Point2<f32>) -> Ray {
    let p_near = camera.unproject(&na::Point3::new(window_pos.x, window_pos.y, -1.0));
    let p_far = camera.unproject(&na::Point3::new(window_pos.x, window_pos.y, 1.0));

    Ray {
        origin: *eye,
        velocity: p_far - p_near,
    }
}

pub fn pick_in_layer(
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

    if let Some((ray_t, _plane_pos)) = ray_quad_intersection(&ray, &quad) {
        let ray_pos = ray.origin + ray_t * ray.velocity;
        let grid_pos = grid::Point3::new(
            ray_pos.x.floor() as isize,
            ray_pos.y.floor() as isize,
            layer,
        );

        if machine.is_valid_pos(&grid_pos) {
            // Intersection
            Some(grid_pos)
        } else {
            // Intersection at invalid position
            None
        }
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
