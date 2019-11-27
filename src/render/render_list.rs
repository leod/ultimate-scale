use crate::render::shader::ToUniforms;
use crate::render::{Object, Resources};

#[derive(Clone, Debug)]
pub struct Instance<T> {
    pub object: Object,
    pub params: T,
}

#[derive(Default, Clone)]
pub struct RenderList<T> {
    pub instances: Vec<Instance<T>>,
}

impl<T: Clone> RenderList<T> {
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

    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

impl<T: ToUniforms> RenderList<T> {
    pub fn render<S: glium::Surface, C: ToUniforms>(
        &self,
        resources: &Resources,
        context: &C,
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
            let uniforms = (&context, &instance.params);

            buffers.index_buffer.draw(
                &buffers.vertex_buffer,
                &program,
                &uniforms.to_uniforms(),
                &params,
                target,
            )?;
        }

        Ok(())
    }
}
