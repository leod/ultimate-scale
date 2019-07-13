mod machine;
mod object;

use nalgebra as na;
use glium;

pub use object::{Object, Instance, InstanceParams};
use object::ObjectBuffers;

pub struct Resources {
    object_buffers: Vec<ObjectBuffers>
}

impl Resources {
    pub fn create<F: glium::backend::Facade>(
        facade: &F
    ) -> Result<Resources, object::CreationError> {
        // Unfortunately, it doesn't seem easy to use enum_map here,
        // since we need to check for errors in creating buffers
        let mut object_buffers = Vec::new();

        for i in 0 .. Object::NumTypes as u32 {
            // Safe to unwrap here, since we iterate within the range
            let object: Object = num_traits::FromPrimitive::from_u32(i).unwrap();

            object_buffers.push(object.create_buffers(facade)?);
        }

        Ok(Resources { object_buffers })
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
}
