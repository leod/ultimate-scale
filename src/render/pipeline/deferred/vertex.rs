use glium::implement_vertex;

#[derive(Copy, Clone)]
pub struct QuadVertex {
    position: [f32; 4],
    tex_coord: [f32; 2],
}

implement_vertex!(QuadVertex, position, tex_coord);

pub const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: [0.0, 0.0, 0.0, 1.0],
        tex_coord: [0.0, 0.0],
    },
    QuadVertex {
        position: [1.0, 0.0, 0.0, 1.0],
        tex_coord: [1.0, 0.0],
    },
    QuadVertex {
        position: [1.0, 1.0, 0.0, 1.0],
        tex_coord: [1.0, 1.0],
    },
    QuadVertex {
        position: [0.0, 1.0, 0.0, 1.0],
        tex_coord: [0.0, 1.0],
    },
];

pub const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];
