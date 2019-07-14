use log::debug;

use nalgebra as na;

use glutin::{VirtualKeyCode, WindowEvent};

use crate::util::intersection::{ray_plane_intersection, Ray, Plane};
use crate::machine::grid;
use crate::machine::Machine;
use crate::render::Camera;

pub struct Editor {
    machine: Machine,

    mouse_window_pos: na::Point2<f32>,
    mouse_grid_pos: Option<grid::Vec3>,
}

impl Editor {
    pub fn new(size: grid::Vec3) -> Editor {
        Editor {
            machine: Machine::new(size),
            mouse_window_pos: na::Point2::origin(),
            mouse_grid_pos: None,
        }
    }

    pub fn update(&mut self, dt_secs: f32, camera: &Camera) {
        let p = self.mouse_window_pos;
        let p_near = camera.unproject(&na::Point3::new(p.x, p.y, -1.0));
        let p_far = camera.unproject(&na::Point3::new(p.x, p.y, 1.0));

        let ray = Ray {
            origin: camera.eye(),
            velocity: p_far - p_near,
        };
        let plane = Plane {
            origin: na::Point3::origin(),   
            direction_a: na::Vector3::x(),
            direction_b: na::Vector3::y(),
        };

        let intersection = ray_plane_intersection(&ray, &plane);
        if let Some((ray_t, plane_pos)) = intersection {
           debug!("ray-plane at {}, {:?}", ray_t, plane_pos);
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
                modifiers: _,
            } => {
                self.mouse_window_pos = na::Point2::new(
                    position.x as f32,
                    position.y as f32,
                );
            }

            _ => ()
        }
    }
}
