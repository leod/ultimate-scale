mod machine;
mod object;
mod camera;

use nalgebra as na;
use glium::{self, program, uniform};
use num_traits::ToPrimitive;

use object::{Object, ObjectBuffers, Instance, InstanceParams};

pub struct Resources {
    object_buffers: Vec<ObjectBuffers>,
    program: glium::Program,
}

pub enum CreationError {
    ObjectCreationError(object::CreationError),
    ProgramChooserCreationError(glium::program::ProgramChooserCreationError),
}

impl From<object::CreationError> for CreationError {
    fn from(err: object::CreationError) -> CreationError {
        CreationError::ObjectCreationError(err)
    }
}

impl From<glium::program::ProgramChooserCreationError> for CreationError {
    fn from(err: glium::program::ProgramChooserCreationError) -> CreationError {
        CreationError::ProgramChooserCreationError(err)
    }
}

impl Resources {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
    ) -> Result<Resources, CreationError> {
        // Unfortunately, it doesn't seem easy to use enum_map here,
        // since we need to check for errors in creating buffers
        let mut object_buffers = Vec::new();

        for i in 0 .. Object::NumTypes as u32 {
            // Safe to unwrap here, since we iterate within the range
            let object: Object = num_traits::FromPrimitive::from_u32(i).unwrap();

            object_buffers.push(object.create_buffers(facade)?);
        }

        let program = program!(facade,
            140 => {
                vertex: "
                    #version 140

                    uniform mat4 mat_projection;
                    uniform mat4 mat_view;
                    uniform mat4 mat_model;

                    in vec2 position;
                    in vec3 color;
                    out vec3 v_color;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0)
                            * mat_projection
                            * mat_view
                            * mat_model;

                        v_color = color;
                    }
                ",

                fragment: "
                    #version 140

                    in vec3 v_color;
                    out vec4 f_color;

                    void main() {
                        f_color = vec4(v_color, 1.0);
                    }
                "
            },
        )?;

        Ok(Resources {
            object_buffers,
            program
        })
    }

    fn get_object_buffers(&self, object: Object) -> &ObjectBuffers {
        // Safe to unwrap array access here, since we have initialized buffers
        // for all objects
        &self.object_buffers[object.to_usize().unwrap()]
    }
}

#[derive(Default)]
pub struct RenderList {
    instances: Vec<Instance>,
}

impl RenderList {
    pub fn add_instance(&mut self, instance: &Instance) {
        self.instances.push(instance.clone());
    }

    pub fn add(&mut self, object: Object, params: &InstanceParams) {
        self.add_instance(&Instance { object, params: params.clone() });
    }

    pub fn render<S: glium::Surface>(
        &self,
        resources: &Resources,
        target: &mut S,
        camera: &camera::Camera,
    ) -> Result<(), glium::DrawError> {
        // TODO: Could sort by object here to reduce state switching for large
        // numbers of objects.

        let mat_projection: [[f32; 4]; 4] = camera.projection.into();
        let mat_view: [[f32; 4]; 4] = camera.view.to_homogeneous().into();

        for instance in &self.instances {
            let buffers = resources.get_object_buffers(instance.object);

            let mat_model: [[f32; 4]; 4] = instance.params.transform.into();
            let uniforms = uniform! {
                mat_projection: mat_projection,
                mat_view: mat_view,
                mat_model: mat_model, 
            };

            target.draw(
                &buffers.vertices,
                &buffers.indices,
                &resources.program,
                &uniforms,
                &Default::default()
            )?;
        }

        Ok(())
    }
}
