use std::path::Path;

use glium::{self, implement_vertex};
use log::info;
use num_derive::{FromPrimitive, ToPrimitive};

pub use crate::render::CreationError;

#[derive(Copy, Clone, PartialEq, Eq, Debug, FromPrimitive, ToPrimitive)]
pub enum Object {
    Triangle,
    Quad,
    Cube,
    LineX,
    LineY,
    LineZ,

    TessellatedCube,
    TessellatedCylinder,

    PipeSegment,
    PipeSplit,
    PipeBend,

    /// Counter of the number of objects
    NumTypes,
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

implement_vertex!(Vertex, position, normal);

pub enum IndexBuffer {
    IndexBuffer(glium::index::IndexBuffer<u32>),
    NoIndices(glium::index::NoIndices),
}

impl IndexBuffer {
    pub fn draw<'a, V, U, S>(
        &self,
        vertex_buffer: V,
        program: &glium::Program,
        uniforms: &U,
        draw_parameters: &glium::DrawParameters,
        target: &mut S,
    ) -> Result<(), glium::DrawError>
    where
        V: glium::vertex::MultiVerticesSource<'a>,
        U: glium::uniforms::Uniforms,
        S: glium::Surface,
    {
        match &self {
            IndexBuffer::IndexBuffer(buffer) => {
                target.draw(vertex_buffer, buffer, program, uniforms, draw_parameters)
            }
            IndexBuffer::NoIndices(buffer) => {
                target.draw(vertex_buffer, buffer, program, uniforms, draw_parameters)
            }
        }
    }
}

pub struct ObjectBuffers {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: IndexBuffer,
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
            .map(|(&p, &n)| Vertex {
                position: p,
                normal: n,
            })
            .collect::<Vec<_>>();

        Ok(ObjectBuffers {
            vertex_buffer: glium::VertexBuffer::new(facade, &vertices)?,
            index_buffer: IndexBuffer::IndexBuffer(glium::IndexBuffer::new(
                facade,
                primitive_type,
                indices,
            )?),
        })
    }

    pub fn load_wavefront<F: glium::backend::Facade>(
        facade: &F,
        path: &Path,
    ) -> Result<ObjectBuffers, CreationError> {
        info!("Loading Wavefront .OBJ file: `{}'", path.display());

        // As in:
        // https://github.com/glium/glium/blob/master/examples/support/mod.rs

        let data = obj::Obj::load(path).unwrap();

        let mut vertices = Vec::new();

        for object in data.objects.iter() {
            for polygon in object.groups.iter().flat_map(|g| g.polys.iter()) {
                match polygon {
                    genmesh::Polygon::PolyTri(genmesh::Triangle {
                        x: v1,
                        y: v2,
                        z: v3,
                    }) => {
                        for v in [v1, v2, v3].iter() {
                            let position = data.position[v.0];
                            let normal = v.2.map(|index| data.normal[index]);

                            let normal = normal.unwrap_or([0.0, 0.0, 0.0]);

                            vertices.push(Vertex { position, normal })
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        }

        let vertex_buffer = glium::VertexBuffer::new(facade, &vertices).unwrap();
        let primitive_type = glium::index::PrimitiveType::TrianglesList;
        let index_buffer = IndexBuffer::NoIndices(glium::index::NoIndices(primitive_type));

        Ok(ObjectBuffers {
            vertex_buffer,
            index_buffer,
        })
    }
}

impl Object {
    #[rustfmt::skip]
    pub fn create_buffers<F: glium::backend::Facade>(
        self, facade: &F
    ) -> Result<ObjectBuffers, CreationError> {
        match self {
            Object::Triangle => {
                let positions = vec![
                    [0.0, -0.5, 0.0],
                    [0.0,  0.5, 0.0],
                    [1.0,  0.0, 0.0],
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
            Object::Quad => {
                let positions = vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                ];

                let normals = vec![
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ];

                let indices = vec![
                    0, 1, 2,
                    2, 3, 0,
                ];

                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    &positions,
                    &normals,
                    &indices,
                )
            }
            Object::Cube => {
                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    CUBE_POSITIONS,
                    CUBE_NORMALS,
                    CUBE_INDICES,
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
            Object::TessellatedCube => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();

                // Number of subdivisions along the x axis
                let n = 64;

                for i in 0..n {
                    // Add one x-slice of the cube
                    let x_offset = i as f32 / n as f32 - 0.5;

                    for (&position, &normal) in CUBE_POSITIONS.iter().zip(CUBE_NORMALS.iter()) {
                        positions.push([x_offset + position[0] / n as f32, position[1], position[2]]);
                        normals.push(normal);
                    }

                    for &index in CUBE_INDICES {
                        indices.push((CUBE_POSITIONS.len() * i) as u32 + index);
                    }
                }

                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    &positions,
                    &normals,
                    &indices,
                )
            }
            Object::TessellatedCylinder => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();

                // Number of subdivisions along the x axis
                let n = 64;

                // Number of subdivisions along the angle
                let m = 12;

                for i in 0..n {
                    let x_left = i as f32 / n as f32 - 0.5;
                    let x_right = (i + 1) as f32 / n as f32 - 0.5;

                    // Add one x-slice of the cylinder
                    for j in 0..m {
                        // Add one stripe of the cylinder
                        let theta = j as f32 / m as f32 * 2.0 * std::f32::consts::PI;
                        let theta_next = (j + 1) as f32 / m as f32 * 2.0 * std::f32::consts::PI;

                        let y = theta.sin();
                        let z = theta.cos();
                        let y_next = theta_next.sin();
                        let z_next = theta_next.cos();

                        let index = positions.len() as u32;

                        positions.push([x_left, y, z]);
                        positions.push([x_right, y, z]);
                        positions.push([x_right, y_next, z_next]);
                        positions.push([x_left, y_next, z_next]);

                        normals.push([0.0, y, z]);
                        normals.push([0.0, y, z]);
                        normals.push([0.0, y_next, z_next]);
                        normals.push([0.0, y_next, z_next]);

                        indices.push(index + 2);
                        indices.push(index + 1);
                        indices.push(index + 0);
                        indices.push(index + 0);
                        indices.push(index + 3);
                        indices.push(index + 2);
                    }
                }
                ObjectBuffers::from_slices(
                    facade,
                    glium::index::PrimitiveType::TrianglesList,
                    &positions,
                    &normals,
                    &indices,
                )
            }
            Object::PipeSegment =>
                ObjectBuffers::load_wavefront(facade, Path::new("resources/pipe_seg.obj")),
            Object::PipeSplit =>
                ObjectBuffers::load_wavefront(facade, Path::new("resources/pipe_split.obj")),
            Object::PipeBend =>
                ObjectBuffers::load_wavefront(facade, Path::new("resources/pipe_bend.obj")),
            Object::NumTypes => panic!("Object::NumTypes cannot be instantiated!"),
        }
    }
}

#[rustfmt::skip]
pub const CUBE_POSITIONS: &[[f32; 3]] = &[
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

#[rustfmt::skip]
pub const CUBE_NORMALS: &[[f32; 3]] = &[
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

#[rustfmt::skip]
pub const CUBE_INDICES: &[u32] = &[
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
