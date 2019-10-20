use nalgebra as na;

use glium::uniform;

use crate::render::pipeline::InstanceParams;

#[derive(Debug, Clone)]
pub struct Light {
    pub position: na::Point3<f32>,
    pub attenuation: na::Vector3<f32>,
    pub color: na::Vector3<f32>,
    pub radius: f32,
}

impl InstanceParams for Light {
    type U = impl glium::uniforms::Uniforms;

    fn uniforms(&self) -> Self::U {
        let position: [f32; 3] = self.position.coords.into();
        let attenuation: [f32; 3] = self.attenuation.into();
        let color: [f32; 3] = self.color.into();

        uniform! {
            light_position: position,
            light_attenuation: attenuation,
            light_color: color,
            light_radius: self.radius,
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            position: na::Point3::origin(),
            attenuation: na::Vector3::zeros(),
            color: na::Vector3::zeros(),
            radius: 1.0,
        }
    }
}
