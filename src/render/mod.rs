mod machine;
mod object;

use nalgebra as na;
use glium::{self, program};

pub use object::{Object, Instance, InstanceParams};
use object::ObjectBuffers;

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
					uniform mat4 matrix;
					in vec2 position;
					in vec3 color;
					out vec3 vColor;
					void main() {
						gl_Position = vec4(position, 0.0, 1.0) * matrix;
						vColor = color;
					}
				",

				fragment: "
					#version 140
					in vec3 vColor;
					out vec4 f_color;
					void main() {
						f_color = vec4(vColor, 1.0);
					}
				"
			},
		)?;

        Ok(Resources {
			object_buffers,
			program
        })
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
        target: &S,
        resources: &Resources,
    ) -> Result<(), glium::DrawError> {
        //target.draw(
        Ok(())
    }
}
