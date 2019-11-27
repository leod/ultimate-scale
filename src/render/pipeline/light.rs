use nalgebra as na;

#[derive(Debug, Clone)]
pub struct Light {
    pub position: na::Point3<f32>,
    pub attenuation: na::Vector3<f32>,
    pub color: na::Vector3<f32>,
    pub is_main: bool,
    pub radius: f32,
}

to_uniforms_impl!(
    Light,
    self => {
        light_position: Vec3 => self.position.coords.into(),
        light_attenuation: Vec3 => self.attenuation.into(),
        light_color: Vec3 => self.color.into(),
        light_is_main: Bool => self.is_main,
        light_radius: Float => self.radius,
    },
);

impl Default for Light {
    fn default() -> Self {
        Self {
            position: na::Point3::origin(),
            attenuation: na::Vector3::zeros(),
            color: na::Vector3::zeros(),
            is_main: false,
            radius: 0.0,
        }
    }
}
