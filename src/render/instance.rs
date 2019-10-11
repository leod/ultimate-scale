use std::fmt::Debug;

use nalgebra as na;
use glium::uniform;

use crate::render::{Object, Context};

pub trait InstanceParams: Clone + Debug + Default {
    type T: glium::uniforms::AsUniformValue;
    type R: glium::uniforms::Uniforms;

    fn uniforms(&self, context: &Context) -> glium::uniforms::UniformsStorage<Self::T, Self::R>;
}

#[derive(Clone, Debug)]
pub struct DefaultInstanceParams {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
}

impl Default for DefaultInstanceParams {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl InstanceParams for DefaultInstanceParams {
    type T = impl glium::uniforms::AsUniformValue;
    type R = impl glium::uniforms::Uniforms;

    fn uniforms(&self, context: &Context) -> glium::uniforms::UniformsStorage<Self::T, Self::R> {
        let mat_projection: [[f32; 4]; 4] = context.camera.projection.into();
        let mat_view: [[f32; 4]; 4] = context.camera.view.into();
        let light_pos: [f32; 3] = context.main_light_pos.coords.into();
        let mat_model: [[f32; 4]; 4] = self.transform.into();
        let color: [f32; 4] = self.color.into();

        uniform! {
            mat_model: mat_model,
            mat_view: mat_view,
            mat_projection: mat_projection,
            light_pos: light_pos,
            color: color,
            t: context.elapsed_time_secs,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance<T: InstanceParams> {
    pub object: Object,
    pub params: T,
}

