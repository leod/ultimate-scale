use glium::uniform;

use nalgebra as na;

use crate::render::pipeline::{Context, InstanceParams};
use crate::render::{object, shader};

#[derive(Debug, Clone)]
pub struct Params {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::zeros(),
        }
    }
}

impl InstanceParams for Params {
    type U = impl glium::uniforms::Uniforms;

    fn uniforms(&self) -> Self::U {
        let mat_model: [[f32; 4]; 4] = self.transform.into();
        let color: [f32; 4] = self.color.into();

        uniform! {
            mat_model: mat_model,
            color: color,
        }
    }
}

pub fn core() -> shader::Core<(Context, Params), object::Vertex> {
    let vertex = shader::VertexCore {
        out_defs: vec![shader::v_world_normal_def(), shader::v_world_pos_def()],
        out_exprs: shader_out_exprs! {
            shader::V_WORLD_NORMAL => "normalize(transpose(inverse(mat3(mat_model))) * normal)",
            shader::V_WORLD_POS => "mat_model * vec4(position * 0.5, 1.0) + vec4(v_world_normal * sin(tick_progress * 3.141592) * 0.1, 0.0)",
            shader::V_POSITION => "mat_projection * mat_view * v_world_pos",
        },
        ..Default::default()
    };

    let fragment = shader::FragmentCore {
        out_defs: vec![shader::f_color_def()],
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "color",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}
