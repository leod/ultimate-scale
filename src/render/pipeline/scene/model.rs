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

to_uniforms_impl!(
    Params,
    self => {
        mat_model: Mat4 => self.transform.into(),
        color: Vec4 => self.color.into(),
    },
);

pub fn scene_core() -> shader::Core<(Context, Params), object::Vertex> {
    shader::Core {
        vertex: shader::VertexCore {
            out_defs: vec![shader::defs::v_world_normal(), shader::defs::v_world_pos()],
            out_exprs: shader_out_exprs! {
                // TODO: Precompute inverse of mat_model if we ever have lots of vertices
                shader::defs::V_WORLD_NORMAL => "normalize(transpose(inverse(mat3(mat_model))) * normal)",
                shader::defs::V_WORLD_POS => "mat_model * vec4(position, 1.0)",
                shader::defs::V_POSITION => "mat_projection * mat_view * v_world_pos",
            },
            ..Default::default()
        },
        fragment: shader::FragmentCore {
            out_defs: vec![shader::defs::f_color()],
            out_exprs: shader_out_exprs! {
                shader::defs::F_COLOR => "color",
            },
            ..Default::default()
        },
    }
}
