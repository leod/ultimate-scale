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
    pub fn add(&mut self, object: Object, params: &T) {
        self.instances.push(Instance {
            object,
            params: params.clone(),
        });
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

impl<T: ToUniforms> RenderList<T> {
    pub fn draw<S: glium::Surface, C: ToUniforms>(
        &self,
        resources: &Resources,
        context: &C,
        program: &glium::Program,
        params: &glium::DrawParameters,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
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
