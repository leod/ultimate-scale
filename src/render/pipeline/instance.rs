use std::fmt::Debug;

use nalgebra as na;

use glium::uniform;
use glium::uniforms::{EmptyUniforms, UniformValue, Uniforms};

use crate::render::pipeline::Context;
use crate::render::Object;

pub trait InstanceParams: Clone + Debug {
    type U: Uniforms;

    fn uniforms(&self) -> Self::U;
}

impl InstanceParams for () {
    type U = EmptyUniforms;

    fn uniforms(&self) -> Self::U {
        EmptyUniforms
    }
}

pub struct UniformsPair<T: Uniforms, U: Uniforms>(pub T, pub U);

impl<T: Uniforms, U: Uniforms> Uniforms for UniformsPair<T, U> {
    fn visit_values<'a, F>(&'a self, mut output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        // F is not Copy, so we have to wrap into a lambda here
        self.0.visit_values(|x, y| output(x, y));
        self.1.visit_values(|x, y| output(x, y));
    }
}

impl InstanceParams for Context {
    type U = impl Uniforms;

    fn uniforms(&self) -> Self::U {
        let mat_view: [[f32; 4]; 4] = self.camera.view.into();
        let mat_projection: [[f32; 4]; 4] = self.camera.projection.into();
        let light_pos: [f32; 3] = self.main_light_pos.coords.into();

        uniform! {
            mat_view: mat_view,
            mat_projection: mat_projection,
            light_pos: light_pos,
            t: self.elapsed_time_secs,
            tick_progress: self.tick_progress,
        }
    }
}

impl<T: InstanceParams, U: InstanceParams> InstanceParams for (T, U) {
    type U = UniformsPair<T::U, U::U>;

    fn uniforms(&self) -> Self::U {
        UniformsPair(self.0.uniforms(), self.1.uniforms())
    }
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
    type U = impl Uniforms;

    fn uniforms(&self) -> Self::U {
        let mat_model: [[f32; 4]; 4] = self.transform.into();
        let color: [f32; 4] = self.color.into();

        uniform! {
            mat_model: mat_model,
            color: color,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance<T: InstanceParams> {
    pub object: Object,
    pub params: T,
}
