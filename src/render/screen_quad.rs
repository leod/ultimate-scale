use glium::implement_vertex;

use crate::render::CreationError;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 4],
    tex_coord: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coord);

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0, 0.0, 1.0],
        tex_coord: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0, 1.0],
        tex_coord: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0, 1.0],
        tex_coord: [0.0, 1.0],
    },
];

pub const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

pub struct ScreenQuad {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: glium::IndexBuffer<u16>,
}

impl ScreenQuad {
    pub fn create<F: glium::backend::Facade>(facade: &F) -> Result<Self, CreationError> {
        let vertex_buffer = glium::VertexBuffer::new(facade, VERTICES)?;
        let index_buffer =
            glium::IndexBuffer::new(facade, glium::index::PrimitiveType::TrianglesList, INDICES)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
        })
    }
}
