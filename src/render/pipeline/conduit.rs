use glium::uniform;

use nalgebra as na;

use crate::render::pipeline::{Context, InstanceParams};
use crate::render::{object, shader};

#[derive(Debug, Clone)]
pub struct Params {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
    pub phase: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::zeros(),
            phase: 0.0,
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
            phase: self.phase,
        }
    }
}

pub fn core() -> shader::Core<(Context, Params), object::Vertex> {
    let vertex = shader::VertexCore {
        out_defs: vec![shader::v_world_normal_def(), shader::v_world_pos_def()],
        defs: "
            const float PI = 3.141592;
            const float radius = 0.5;
            const float scale = 0.15;
        "
        .to_string(),
        body: "
            float a = (position.x + 0.5 + tick_progress) * 2.0 * PI + phase;
            float s = sin(a);
            float c = cos(a);
            mat2 m = mat2(c, -s, s, c);

            vec3 scaled_pos = position;
            scaled_pos.yz *= scale;
            scaled_pos.z += radius - 0.5 * scale;
            scaled_pos.yz = m * scaled_pos.yz;

            vec3 rot_normal = normal;
            rot_normal.yz = m * rot_normal.yz;
        "
        .to_string(),
        out_exprs: shader_out_exprs! {
            shader::V_WORLD_NORMAL => "normalize(transpose(inverse(mat3(mat_model))) * rot_normal)",
            shader::V_WORLD_POS => "mat_model * vec4(scaled_pos, 1.0)",
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
