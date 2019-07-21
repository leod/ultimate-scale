use log::info;

use glium::program;
use num_traits::ToPrimitive;

use crate::render::object::{self, Object, ObjectBuffers};

pub struct Resources {
    pub object_buffers: Vec<ObjectBuffers>,
    pub program: glium::Program,
    pub plain_program: glium::Program,
}

#[derive(Debug)]
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
    pub fn create<F: glium::backend::Facade>(facade: &F) -> Result<Resources, CreationError> {
        // Unfortunately, it doesn't seem easy to use enum_map here,
        // since we need to check for errors in creating buffers
        let mut object_buffers = Vec::new();

        for i in 0..Object::NumTypes as u32 {
            // Safe to unwrap here, since we iterate within the range
            let object: Object = num_traits::FromPrimitive::from_u32(i).unwrap();

            object_buffers.push(object.create_buffers(facade)?);
        }

        info!("Creating straight render program");
        let program = program!(facade,
            140 => {
                vertex: "
                    #version 140

                    uniform mat4 mat_model;
                    uniform mat4 mat_view;
                    uniform mat4 mat_projection;

                    uniform vec4 color;

                    in vec3 position;
                    in vec3 normal;

                    out vec3 v_world_normal;
                    out vec4 v_world_pos;
                    out vec4 v_color;

                    void main() {
                        v_world_pos = mat_model * vec4(position, 1.0);

                        gl_Position = mat_projection
                            * mat_view
                            * v_world_pos;

                        v_world_normal = mat3(mat_model) * normal;
                        v_color = color;
                    }
                ",

                fragment: "
                    #version 140

                    uniform vec3 light_pos;

                    in vec4 v_world_pos;
                    in vec3 v_world_normal;
                    in vec4 v_color;
                    out vec4 f_color;

                    void main() {
                        float ambient = 0.3;
                        float diffuse = max(
                            dot(
                                normalize(v_world_normal),
                                normalize(light_pos - v_world_pos.xyz)
                            ),
                            0.05
                        );

                        f_color = (ambient + diffuse) * v_color;
                    }
                "
            },
        )?;

        info!("Creating plain render program");
        let plain_program = program!(facade,
            140 => {
                vertex: "
                    #version 140

                    uniform mat4 mat_model;
                    uniform mat4 mat_view;
                    uniform mat4 mat_projection;

                    uniform vec4 color;

                    in vec3 position;
                    out vec4 v_color;

                    void main() {
                        gl_Position = mat_projection
                            * mat_view
                            * mat_model
                            * vec4(position, 1.0);

                        v_color = color;
                    }
                ",

                fragment: "
                    #version 140

                    uniform float t;

                    in vec4 v_color;
                    out vec4 f_color;

                    void main() {
                        f_color = v_color;
                    }
                "
            },
        )?;

        Ok(Resources {
            object_buffers,
            program,
            plain_program,
        })
    }

    pub fn get_object_buffers(&self, object: Object) -> &ObjectBuffers {
        // Safe to unwrap array access here, since we have initialized buffers
        // for all objects
        &self.object_buffers[object.to_usize().unwrap()]
    }
}
