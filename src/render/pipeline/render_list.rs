use crate::render::pipeline::instance::UniformsPair;
use crate::render::pipeline::{Context, Instance, InstanceParams};
use crate::render::{Object, Resources};

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
        // TODO: Fix cylinder so that we can reenable backface culling
        let params = glium::DrawParameters {
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            ..params.clone()
        };

        for instance in &self.instances {
            let buffers = resources.get_object_buffers(instance.object);
            let uniforms = UniformsPair(context.uniforms(), instance.params.uniforms());

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
