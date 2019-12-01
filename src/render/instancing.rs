use std::ops::Range;

use log::info;

use num_traits::{FromPrimitive, ToPrimitive};

use crate::render::shader::ToVertex;
use crate::render::{Instance, Object, Resources};

pub use crate::render::error::{CreationError, DrawError};

pub const INSTANCES_PER_BUFFER: usize = 10000;

struct Entry {
    object: Object,
    range: Range<usize>,
}

struct Buffer<V: glium::vertex::Vertex> {
    buffer: glium::VertexBuffer<V>,
    entries: Vec<Entry>,
}

impl<V> Buffer<V>
where
    V: glium::vertex::Vertex,
{
    fn create<F: glium::backend::Facade>(facade: &F) -> Result<Self, CreationError> {
        let buffer = glium::VertexBuffer::empty_dynamic(facade, INSTANCES_PER_BUFFER)?;

        Ok(Self {
            buffer,
            entries: Vec::new(),
        })
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn remaining_capacity(&self) -> usize {
        if let Some(last) = self.entries.last() {
            assert!(last.range.end <= self.buffer.len());

            self.buffer.len() - last.range.end
        } else {
            self.buffer.len()
        }
    }

    fn append(&mut self, object: Object, vertices: &[V]) -> usize {
        let capacity = self.remaining_capacity();

        if capacity == 0 {
            return 0;
        }

        let start_index = self.buffer.len() - capacity;
        let num_to_write = vertices.len().min(capacity);
        let range = start_index..start_index + num_to_write;

        // Safe to unwrap since we have bounded the range to our capacity.
        let slice = self.buffer.slice_mut(range.clone()).unwrap();
        slice.write(&vertices[0..num_to_write]);

        self.entries.push(Entry { object, range });

        num_to_write
    }

    fn draw<S, U>(
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

            // Safe to unwrap here, since range is given.
            let instances = self.buffer.slice(entry.range.clone()).unwrap();

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

pub struct Instancing<I: ToVertex> {
    buffers: Vec<Buffer<I::Vertex>>,
    buckets: Vec<Vec<I::Vertex>>,
}

impl<I> Instancing<I>
where
    I: ToVertex + Clone,
{
    pub fn create<F: glium::backend::Facade>(facade: &F) -> Result<Self, CreationError> {
        let buffers = vec![Buffer::create(facade)?];

        let mut buckets = Vec::new();
        for _ in 0..Object::NumTypes as usize {
            buckets.push(Vec::new());
        }

        Ok(Self { buffers, buckets })
    }

    pub fn update<'a, F: glium::backend::Facade>(
        &'a mut self,
        facade: &F,
        instances: &[Instance<I>],
        //instances: impl Iterator<Item=&'a Instance<I>>,
    ) -> Result<(), CreationError> {
        // Sort the instances into buckets according to their Object (i.e. mesh)
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        for instance in instances {
            // Safe to unwrap here.
            let index = instance.object.to_usize().unwrap();

            self.buckets[index].push(instance.params.to_vertex());
        }

        // Write instance data into vertex buffers. We move through the buffers
        // that we have, filling them up sequentially.
        for buffer in &mut self.buffers {
            buffer.clear();
        }

        let mut cur_buffer = 0;

        for i in 0..Object::NumTypes as usize {
            if self.buckets[i].is_empty() {
                continue;
            }

            // Safe to unwrap here, since we iterate within the range.
            let object: Object = FromPrimitive::from_usize(i).unwrap();

            let mut vertices = &self.buckets[i][0..];

            while !vertices.is_empty() {
                // Write as much as possible into the current buffer.
                let num_written = self.buffers[cur_buffer].append(object, vertices);

                if num_written == 0 {
                    // We had vertex data to write, but nothing was written
                    // Move on to the next buffer.
                    cur_buffer += 1;

                    if cur_buffer == self.buffers.len() {
                        // We have reached past the last buffer. Create a new
                        // buffer to write into.
                        self.buffers.push(Buffer::create(facade)?);

                        info!(
                            "Created new vertex buffer for I={}.\
                             Have {} instances of type {:?}, and now {} buffers.",
                            std::any::type_name::<I>(),
                            self.buckets[i].len(),
                            object,
                            cur_buffer + 1,
                        );
                    }
                } else {
                    // We have written something into the buffer, reduce slice
                    // accordingly.
                    vertices = &vertices[num_written..];
                }
            }
        }

        Ok(())
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
        for buffer in self.buffers.iter() {
            buffer.draw(resources, program, uniforms, params, target)?;
        }

        Ok(())
    }
}
