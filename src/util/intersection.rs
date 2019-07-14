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

/// Determine the intersection between a ray and a plane.
/// The results are the ray's time of impact as well as the coordinates of the
/// intersection in terms of the plane's `direction_a` and `direction_b`, or 
/// `None` if the plane is ill-defined.
pub fn ray_plane_intersection(
    ray: &Ray,
    plane: &Plane,
) -> Option<(f32, na::Vector2<f32>)> {
    let matrix = na::Matrix3::from_columns(&[
        ray.velocity,
        -plane.direction_a,
        -plane.direction_b,
    ]);

    let inverse = matrix.try_inverse()?;
    let solution = inverse * (plane.origin - ray.origin);

    Some((solution.x, na::Vector2::new(solution.y, solution.z)))
}
