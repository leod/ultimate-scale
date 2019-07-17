use glium::uniform;
use nalgebra as na;

use crate::render::camera::Camera;
use crate::render::object::{self, Instance, InstanceParams, Object};
use crate::render::resources::Resources;
use crate::render::Context;

#[derive(Default, Clone)]
pub struct RenderList {
    pub instances: Vec<Instance>,
}

impl RenderList {
    pub fn new() -> RenderList {
        Default::default()
    }

    pub fn add_instance(&mut self, instance: &Instance) {
        self.instances.push(instance.clone());
    }

    pub fn add(&mut self, object: Object, params: &InstanceParams) {
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
        let mat_projection: [[f32; 4]; 4] = context.camera.projection.into();
        let mat_view: [[f32; 4]; 4] = context.camera.view.into();

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

            let mat_model: [[f32; 4]; 4] = instance.params.transform.into();
            let color: [f32; 4] = instance.params.color.into();
            let uniforms = uniform! {
                mat_model: mat_model,
                mat_view: mat_view,
                mat_projection: mat_projection,
                color: color,
                t: context.elapsed_time_secs,
            };

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
