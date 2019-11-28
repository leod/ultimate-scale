use nalgebra as na;

use crate::render::object;
use crate::render::pipeline::Context;
use crate::render::shader;

#[derive(Clone, Debug)]
pub struct Params {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl_uniform_input_and_to_vertex!(
    Params,
    self => {
        mat_model: Mat4 => self.transform.into(),
        color: Vec4 => self.color.into(),
    },
);

pub fn scene_core() -> shader::Core<Context, Params, object::Vertex> {
    let vertex = shader::VertexCore::empty()
        .with_out(
            // TODO: Precompute inverse of mat_model if we ever have lots of vertices
            shader::defs::v_world_normal(),
            "normalize(transpose(inverse(mat3(mat_model))) * normal)",
        )
        .with_out(
            shader::defs::v_world_pos(),
            "mat_model * vec4(position, 1.0)",
        )
        .with_out(shader::defs::v_color(), "color")
        .with_out_expr(
            shader::defs::V_POSITION,
            "mat_projection * mat_view * v_world_pos",
        );

    let fragment = shader::FragmentCore::default()
        .with_in_def(shader::defs::v_color())
        .with_out(shader::defs::f_color(), "v_color");

    shader::Core { vertex, fragment }
}
