use std::mem;

use nalgebra as na;

pub struct Ray {
    pub origin: na::Point3<f32>,
    pub velocity: na::Vector3<f32>,
}

pub struct Plane {
    pub origin: na::Point3<f32>,
    pub direction_a: na::Vector3<f32>,
    pub direction_b: na::Vector3<f32>,
}

pub struct AABB {
    pub min: na::Point3<f32>,
    pub max: na::Point3<f32>,
}

/// Determine the intersection between a ray and a plane.
/// The results are the ray's time of impact as well as the coordinates of the
/// intersection in terms of the plane's `direction_a` and `direction_b`, or
/// `None` if the plane is ill-defined.
pub fn ray_plane_intersection(ray: &Ray, plane: &Plane) -> Option<(f32, na::Vector2<f32>)> {
    let matrix = na::Matrix3::from_columns(&[ray.velocity, -plane.direction_a, -plane.direction_b]);

    let inverse = matrix.try_inverse()?;
    let solution = inverse * (plane.origin - ray.origin);

    Some((solution.x, na::Vector2::new(solution.y, solution.z)))
}

pub fn ray_quad_intersection(ray: &Ray, plane: &Plane) -> Option<(f32, na::Vector2<f32>)> {
    let (ray_t, plane_pos) = ray_plane_intersection(ray, plane)?;

    if plane_pos.x >= 0.0 && plane_pos.x <= 1.0 && plane_pos.y >= 0.0 && plane_pos.y <= 1.0 {
        Some((ray_t, plane_pos))
    } else {
        None
    }
}

pub fn ray_aabb_intersection(ray: &Ray, aabb: &AABB) -> Option<f32> {
    // As in:
    // https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-box-intersection

    let mut t_min = (aabb.min.x - ray.origin.x) / ray.velocity.x;
    let mut t_max = (aabb.max.x - ray.origin.x) / ray.velocity.x;

    if t_min > t_max {
        mem::swap(&mut t_min, &mut t_max);
    }

    let mut ty_min = (aabb.min.y - ray.origin.y) / ray.velocity.y;
    let mut ty_max = (aabb.max.y - ray.origin.y) / ray.velocity.y;

    if ty_min > ty_max {
        mem::swap(&mut ty_min, &mut ty_max);
    }

    if t_min > ty_max || ty_min > t_max {
        return None;
    }

    if ty_min > t_min {
        t_min = ty_min;
    }

    if ty_max < t_max {
        t_max = ty_max;
    }

    let mut tz_min = (aabb.min.z - ray.origin.z) / ray.velocity.z;
    let mut tz_max = (aabb.max.z - ray.origin.z) / ray.velocity.z;

    if tz_min > tz_max {
        mem::swap(&mut tz_min, &mut tz_max);
    }

    if t_min > tz_max || tz_min > t_max {
        return None;
    }

    if tz_min > t_min {
        t_min = tz_min;
    }

    /*if tz_max < t_max {
        t_max = tz_max;
    }*/

    Some(t_min)
}
