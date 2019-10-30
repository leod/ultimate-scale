use nalgebra as na;

use crate::machine::{grid, Machine};
use crate::render::{Camera, EditCameraView};
use crate::util::intersection::{ray_quad_intersection, Plane, Ray};

pub fn pick_in_layer(
    machine: &Machine,
    layer: isize,
    camera: &Camera,
    edit_camera_view: &EditCameraView,
    window_pos: &na::Point2<f32>,
) -> Option<grid::Point3> {
    let p_near = camera.unproject(&na::Point3::new(window_pos.x, window_pos.y, -1.0));
    let p_far = camera.unproject(&na::Point3::new(window_pos.x, window_pos.y, 1.0));

    let ray = Ray {
        origin: edit_camera_view.eye(),
        velocity: p_far - p_near,
    };

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
