use log::info;

use num_traits::ToPrimitive;

use crate::render::object::{self, Object, ObjectBuffers};
use crate::render::pipeline;

pub struct Resources {
    pub object_buffers: Vec<ObjectBuffers>,
    pub program: glium::Program,
    pub wind_program: glium::Program,
    pub plain_program: glium::Program,
}

#[derive(Debug)]
pub enum CreationError {
    ObjectCreationError(object::CreationError),
    ProgramCreationError(glium::program::ProgramCreationError),
}

impl From<object::CreationError> for CreationError {
    fn from(err: object::CreationError) -> CreationError {
        CreationError::ObjectCreationError(err)
    }
}

impl From<glium::program::ProgramCreationError> for CreationError {
    fn from(err: glium::program::ProgramCreationError) -> CreationError {
        CreationError::ProgramCreationError(err)
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
        let program = pipeline::simple::diffuse_core_transform(pipeline::simple::plain_core())
            .build_program(facade)?;

        info!("Creating straight wind program");
        let wind_core = pipeline::simple::diffuse_core_transform(pipeline::wind::core()).link();
        let wind_program = wind_core.build_program(facade)?;

        info!("Creating plain render program");
        let plain_program = pipeline::simple::plain_core().build_program(facade)?;

        Ok(Resources {
            object_buffers,
            program,
            wind_program,
            plain_program,
        })
    }

    pub fn get_object_buffers(&self, object: Object) -> &ObjectBuffers {
        // Safe to unwrap array access here, since we have initialized buffers
        // for all objects
        &self.object_buffers[object.to_usize().unwrap()]
    }
}
