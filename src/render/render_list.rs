use std::fmt::Debug;

use glium::uniform;
use nalgebra as na;

use crate::render::object::Object;
use crate::render::resources::Resources;
use crate::render::Context;

pub trait InstanceParams: Clone + Debug {
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

#[derive(Default, Clone)]
pub struct RenderList<T: InstanceParams> {
    pub instances: Vec<Instance<T>>,
}

impl<T: InstanceParams> RenderList<T> {
    pub fn new() -> RenderList<T> {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, instance: &Instance<T>) {
        self.instances.push(instance.clone());
    }

    pub fn add(&mut self, object: Object, params: &T) {
        self.add_instance(&Instance {
            object,
            params: params.clone(),
        });
    }

    pub fn render_with_program<S: glium::Surface>(
        &self,
        resources: &Resources,
        context: &Context,
        params: &glium::DrawParameters,
        program: &glium::Program,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        let params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            ..params.clone()
        };

        for instance in &self.instances {
            let buffers = resources.get_object_buffers(instance.object);
            let uniforms = instance.params.uniforms(context);

            buffers.index_buffer.draw(
                &buffers.vertex_buffer,
                &program,
                &uniforms,
                &params,
                target,
            )?;
        }

        Ok(())
    }

    pub fn render<S: glium::Surface>(
        &self,
        resources: &Resources,
        context: &Context,
        params: &glium::DrawParameters,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        self.render_with_program(resources, context, params, &resources.program, target)
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }
}
