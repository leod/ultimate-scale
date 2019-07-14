use num_derive::{FromPrimitive, ToPrimitive};
use nalgebra as na;
use glium::{self, implement_vertex};

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

implement_vertex!(Vertex, position, normal);

pub(in crate::render) struct ObjectBuffers {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: glium::IndexBuffer<u32>,
}

impl ObjectBuffers {
    pub fn from_slices<F: glium::backend::Facade>(
        facade: &F,
        primitive_type: glium::index::PrimitiveType,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
    ) -> Result<ObjectBuffers, CreationError> {
        let vertices = positions
            .iter()
            .zip(normals.iter())
            .map(|(&p, &n)| Vertex { position: p, normal: n })
            .collect::<Vec<_>>();

        Ok(ObjectBuffers {
            vertex_buffer: glium::VertexBuffer::new(facade, &vertices)?,
            index_buffer: glium::IndexBuffer::new(
                facade,
                primitive_type,
                indices,
            )?,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, FromPrimitive, ToPrimitive)]
pub enum Object {
    Triangle,
    Cube,
    LineX,
    LineY,
    LineZ,

    /// Counter of the number of objects
    NumTypes,
}

#[derive(Clone, Debug)]
pub struct InstanceParams {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
}

impl Default for InstanceParams {
    fn default() -> InstanceParams {
        InstanceParams {
            transform: na::Matrix4::identity(),
            color: na::Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub object: Object,
    pub params: InstanceParams,
}

impl Instance {

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
            Object::Triangle => {
                let positions = vec![
                    [1.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ];

                let normals = vec![
                    [0.0, 0.0, -1.0],
                    [0.0, 0.0, -1.0],
                    [0.0, 0.0, -1.0],
                ];

                let indices = vec![0, 1, 2];

                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    &positions,
                    &normals,
                    &indices,
                )
            }
            Object::Cube => {
                let positions = vec![
                    // Front
                    [-0.5, -0.5,  0.5],
                    [ 0.5, -0.5,  0.5],
                    [ 0.5,  0.5,  0.5],
                    [-0.5,  0.5,  0.5],

                    // Right
                    [ 0.5,  0.5, -0.5],
                    [ 0.5,  0.5,  0.5],
                    [ 0.5, -0.5,  0.5],
                    [ 0.5, -0.5, -0.5],

                    // Back
                    [ 0.5, -0.5, -0.5],
                    [-0.5, -0.5, -0.5],
                    [-0.5,  0.5, -0.5],
                    [ 0.5,  0.5, -0.5],

                    // Left
                    [-0.5, -0.5, -0.5],
                    [-0.5, -0.5,  0.5],
                    [-0.5,  0.5,  0.5],
                    [-0.5,  0.5, -0.5],

                    // Top
                    [-0.5,  0.5,  0.5],
                    [ 0.5,  0.5,  0.5],
                    [ 0.5,  0.5, -0.5],
                    [-0.5,  0.5, -0.5],

                    // Bottom
                    [-0.5, -0.5, -0.5],
                    [ 0.5, -0.5, -0.5],
                    [ 0.5, -0.5,  0.5],
                    [-0.5, -0.5,  0.5],
                ];

                let normals = vec![
                    // Front
                    [ 0.0,  0.0,  1.0],
                    [ 0.0,  0.0,  1.0],
                    [ 0.0,  0.0,  1.0],
                    [ 0.0,  0.0,  1.0],

                    // Right
                    [ 1.0,  0.0,  0.0],
                    [ 1.0,  0.0,  0.0],
                    [ 1.0,  0.0,  0.0],
                    [ 1.0,  0.0,  0.0],

                    // Back
                    [ 0.0,  0.0, -1.0],
                    [ 0.0,  0.0, -1.0],
                    [ 0.0,  0.0, -1.0],
                    [ 0.0,  0.0, -1.0],

                    // Left
                    [-1.0,  0.0,  0.0],
                    [-1.0,  0.0,  0.0],
                    [-1.0,  0.0,  0.0],
                    [-1.0,  0.0,  0.0],

                    // Top
                    [ 0.0,  1.0,  0.0],
                    [ 0.0,  1.0,  0.0],
                    [ 0.0,  1.0,  0.0],
                    [ 0.0,  1.0,  0.0],

                    // Bottom
                    [ 0.0, -1.0,  0.0],
                    [ 0.0, -1.0,  0.0],
                    [ 0.0, -1.0,  0.0],
                    [ 0.0, -1.0,  0.0],
                ];

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

                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    &positions,
                    &normals,
                    &indices,
                )
            }
            Object::LineX => {
                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::LinesList,
                    &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                    &[[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                    &[0, 1],
                )
            }
            Object::LineY => {
                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::LinesList,
                    &[[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    &[[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                    &[0, 1],
                )
            }
            Object::LineZ => {
                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::LinesList,
                    &[[0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
                    &[[1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                    &[0, 1],
                )
            }
            Object::NumTypes => panic!("Object::NumTypes cannot be instantiated!"),
        }
    }
}
