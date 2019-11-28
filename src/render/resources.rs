use log::info;

use num_traits::ToPrimitive;

use crate::render::object::{Object, ObjectBuffers};
use crate::render::pipeline;

pub use crate::render::CreationError;

pub struct Resources {
    pub object_buffers: Vec<ObjectBuffers>,
    pub plain_program: glium::Program,
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

        info!("Creating plain render program");
        let plain_program = pipeline::scene::model::scene_core().build_program(facade)?;

        Ok(Resources {
            object_buffers,
            plain_program,
        })
    }

    pub fn get_object_buffers(&self, object: Object) -> &ObjectBuffers {
        // Safe to unwrap array access here, since we have initialized buffers
        // for all objects
        &self.object_buffers[object.to_usize().unwrap()]
    }
}
