use num_derive::{FromPrimitive, ToPrimitive};
use glium::{self, implement_vertex};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
}

implement_vertex!(Vertex, position);

pub(in crate::render) struct ObjectBuffers {
    vertices: glium::VertexBuffer<Vertex>,
    indices: glium::IndexBuffer<u32>,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum Object {
    Cube,

    /// Counter of the number of objects
    NumTypes,
}

#[derive(Copy, Clone, Debug)]
pub enum CreationError {
    VertexBufferCreationError(glium::vertex::BufferCreationError),
    IndexBufferCreationError(glium::index::BufferCreationError),
}

impl From<glium::vertex::BufferCreationError> for CreationError {
    fn from(err: glium::vertex::BufferCreationError) -> CreationError {
        CreationError::VertexBufferCreationError(err)
    }
}

impl From<glium::index::BufferCreationError> for CreationError {
    fn from(err: glium::index::BufferCreationError) -> CreationError {
        CreationError::IndexBufferCreationError(err)
    }
}

impl Object {
    pub(in crate::render) fn create_buffers<F: glium::backend::Facade>(
        &self, facade: &F
    ) -> Result<ObjectBuffers, CreationError> {
        match self {
            Object::Cube => {
                let positions = vec![
                    // Front
                    [-1.0, -1.0,  1.0],
                    [ 1.0, -1.0,  1.0],
                    [ 1.0,  1.0,  1.0],
                    [-1.0,  1.0,  1.0],

                    // Right
                    [ 1.0,  1.0,  1.0],
                    [ 1.0,  1.0, -1.0],
                    [ 1.0, -1.0, -1.0],
                    [ 1.0, -1.0,  1.0],

                    // Back
                    [-1.0, -1.0, -1.0],
                    [ 1.0, -1.0, -1.0],
                    [ 1.0,  1.0, -1.0],
                    [-1.0,  1.0, -1.0],

                    // Left
                    [-1.0, -1.0, -1.0],
                    [-1.0, -1.0,  1.0],
                    [-1.0,  1.0,  1.0],
                    [-1.0,  1.0, -1.0],

                    // Top
                    [-1.0,  1.0,  1.0],
                    [-1.0,  1.0,  1.0],
                    [-1.0,  1.0, -1.0],
                    [ 1.0,  1.0, -1.0],

                    // Bottom
                    [-1.0, -1.0, -1.0],
                    [ 1.0, -1.0, -1.0],
                    [ 1.0, -1.0,  1.0],
                    [-1.0, -1.0,  1.0],
                ];

                let vertices = positions.iter().map(|&p| Vertex { position: p })
                    .collect::<Vec<_>>();

                let indices = vec![
                    // Front
                    0, 1, 2, 0, 2, 3,
                    
                    // Right
                    4, 5, 6, 4, 6, 7,

                    // Back
                    8, 9, 10, 8, 10, 11,

                    // Left
                    12, 13, 14, 12, 14, 15,

                    // Top
                    16, 17, 18, 16, 18, 19,

                    // Bottom
                    20, 21, 22, 20, 22, 23,
                ];

                Ok(ObjectBuffers {
                    vertices: glium::VertexBuffer::new(facade, &vertices)?,
                    indices: glium::IndexBuffer::new(
                        facade,
                        glium::index::PrimitiveType::TrianglesList,
                        &indices,
                    )?,
                })
            }
            Object::NumTypes => panic!("Object::NumTypes cannot be instantiated!"),
        }
    }
}


