use glium::uniform;

use nalgebra as na;

use crate::render::pipeline::{Context, InstanceParams};
use crate::render::{object, shader};

#[derive(Debug, Clone)]
pub struct Params {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
    pub phase: f32,
    pub start: f32,
    pub end: f32,
    pub bend: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::zeros(),
            phase: 0.0,
            start: 0.0,
            end: 0.0,
            bend: false,
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
            start: self.start,
            end: self.end,
            bend: self.bend,
        }
    }
}

const V_DISCARD: &str = "v_discard";

fn v_discard() -> shader::VertexOutDef {
    (
        (V_DISCARD.into(), glium::uniforms::UniformType::Float),
        shader::VertexOutQualifier::Smooth,
    )
}

pub fn scene_core() -> shader::Core<(Context, Params), object::Vertex> {
    let vertex = shader::VertexCore {
        out_defs: vec![
            shader::v_world_normal_def(),
            shader::v_world_pos_def(),
            v_discard(),
        ],
        defs: "
            const float PI = 3.141592;
            const float radius = 0.05;
            const float scale = 0.0075;
        "
        .to_string(),
        body: "
            float angle = (position.x + 0.5 + 2.0 * tick_progress) * 1.0 * PI + phase;
            float rot_s = sin(angle);
            float rot_c = cos(angle);
            mat2 rot_m = mat2(rot_c, -rot_s, rot_s, rot_c);

            vec3 scaled_pos = position;
            scaled_pos.yz *= scale;
            scaled_pos.z += radius;

            if (bend) {
                // Currently unused, for wind bending
                float tau = (0.5 - position.x) * PI / 2.0;
                scaled_pos.x = 0.5 * cos(tau);
                scaled_pos.y += 0.5 * sin(tau);

                vec3 normal = vec3(cos(tau), sin(tau), 0.0);
                scaled_pos += normal * sin(angle) * 0.05;
            }

            //scaled_pos.y += sin(angle) * radius;
            //scaled_pos.z += cos(angle) * radius;

            vec3 rot_normal = normal;
            scaled_pos.yz = rot_m * scaled_pos.yz;
            rot_normal.yz = rot_m * rot_normal.yz;

            if (0.5 - position.x < start || 0.5 - position.x > end)
                v_discard = 1.0;
            else
                v_discard = 0.0;
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
        in_defs: vec![v_discard()],
        out_defs: vec![shader::f_color_def()],
        body: "
            if (v_discard >= 0.5)
                discard;
        "
        .to_string(),
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "color",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}
