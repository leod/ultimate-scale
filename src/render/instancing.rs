use num_traits::{FromPrimitive, ToPrimitive};

use crate::render::shader::{InstanceMode, ToVertex};
use crate::render::{Instance, Object, Resources};

pub use crate::render::error::{CreationError, DrawError};

pub const INSTANCES_PER_BUFFER: usize = 10000;

struct Entry {
    object: Object,
    start: usize,
    len: usize,
}

pub struct Instancing<I: ToVertex> {
    buffer: glium::VertexBuffer<I::Vertex>,
    entries: Vec<Entry>,

    buckets: Vec<Vec<I::Vertex>>,
}

impl<I> Instancing<I>
where
    I: ToVertex + Clone,
{
    pub fn create<F: glium::backend::Facade>(facade: &F) -> Result<Self, CreationError> {
        let buffer = glium::VertexBuffer::empty_dynamic(facade, INSTANCES_PER_BUFFER)?;

        let mut buckets = Vec::new();
        for i in 0..Object::NumTypes as usize {
            buckets.push(Vec::new());
        }

        Ok(Self {
            buffer,
            entries: Vec::new(),
            buckets,
        })
    }

    pub fn update(&mut self, instances: &[Instance<I>]) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        for instance in instances {
            // Safe to unwrap here
            let index = instance.object.to_usize().unwrap();

            self.buckets[index].push(instance.params.to_vertex());
        }

        self.entries.clear();

        let mut cur_index = 0;

        for i in 0..Object::NumTypes as usize {
            if !self.buckets[i].is_empty() {
                // Safe to unwrap here, since we iterate within the range
                let object: Object = FromPrimitive::from_usize(i).unwrap();

                let range = cur_index..cur_index + self.buckets[i].len();

                // TODO: Unwrap may trigger here; use multiple buffers as necessary
                let slice = self.buffer.slice_mut(range).unwrap();

                slice.write(&self.buckets[i]);

                self.entries.push(Entry {
                    object,
                    start: cur_index,
                    len: self.buckets[i].len(),
                });

                cur_index += self.buckets[i].len();
            }
        }
    }
}

impl<I> Instancing<I>
where
    I: ToVertex,
{
    pub fn draw<S, U>(
        &self,
        resources: &Resources,
        program: &glium::Program,
        uniforms: &U,
        params: &glium::DrawParameters,
        target: &mut S,
    ) -> Result<(), DrawError>
    where
        S: glium::Surface,
        U: glium::uniforms::Uniforms,
    {
        for entry in self.entries.iter() {
            let buffers = resources.get_object_buffers(entry.object);

            //println!("rendering {} of type {:?}", entry.len, entry.object);

            // Safe to unwrap here since range is given
            let instances = self
                .buffer
                .slice(entry.start..entry.start + entry.len)
                .unwrap();

            // TODO: Fall back to non-instanced rendering if not supported?
            let per_instance = instances
                .per_instance()
                .map_err(|_| DrawError::InstancingNotSupported)?;

            let vertices = (&buffers.vertex_buffer, per_instance);

            buffers
                .index_buffer
                .draw(vertices, program, uniforms, params, target)?;
        }

        Ok(())
    }
}
