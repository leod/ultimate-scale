use nalgebra as na;

use crate::render::pipeline::Context;
use crate::render::{object, shader};

#[derive(Debug, Clone)]
pub struct Params {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
    pub stripe_color: na::Vector4<f32>,
    pub phase: f32,
    pub start: f32,
    pub end: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            transform: na::Matrix4::identity(),
            color: na::Vector4::zeros(),
            stripe_color: na::Vector4::zeros(),
            phase: 0.0,
            start: 0.0,
            end: 0.0,
        }
    }
}

impl_uniform_input_and_to_vertex!(
    Params,
    self => {
        mat_model: Mat4 => self.transform.into(),
        color: Vec4 => self.color.into(),
        stripe_color: Vec4 => self.stripe_color.into(),
        phase: Float => self.phase,
        start: Float => self.start,
        end: Float => self.end,
    },
);

const V_DISCARD: &str = "v_discard";

fn v_discard() -> shader::VertexOutDef {
    (
        (V_DISCARD.into(), glium::uniforms::UniformType::Float),
        shader::VertexOutQualifier::Smooth,
    )
}

const V_COLOR: &str = "v_color";

fn v_color() -> shader::VertexOutDef {
    (
        (V_COLOR.into(), glium::uniforms::UniformType::FloatVec4),
        shader::VertexOutQualifier::Smooth,
    )
}

pub fn scene_core() -> shader::Core<Context, Params, object::Vertex> {
    let vertex = shader::VertexCore::empty()
        .with_defs(
            "
            const float PI = 3.141592;
            const float radius = 0.04;
            const float scale = 0.0105;
        ",
        )
        .with_out_def(v_discard())
        .with_out_def(v_color())
        .with_body(
            "
            float angle = (position.x + 0.5) * PI
                + tick_progress * PI / 2.0
                + phase;

            float rot_s = sin(angle);
            float rot_c = cos(angle);
            mat2 rot_m = mat2(rot_c, -rot_s, rot_s, rot_c);

            vec3 scaled_pos = position;
            scaled_pos.yz *= scale;
            scaled_pos.z += radius;

            vec3 rot_normal = normal;
            scaled_pos.yz = rot_m * scaled_pos.yz;
            rot_normal.yz = rot_m * rot_normal.yz;

            float x = 0.5 - position.x;

            if (x < start || x > end || start == end)
                v_discard = 1.0;
            else
                v_discard = 0.0;

            if (x < tick_progress && x > tick_progress - 0.3)
                v_color = stripe_color;
            else if (end == 1.0 && x > 0.7 + tick_progress)
                v_color = stripe_color;
            else
                v_color = color;
        ",
        )
        .with_out(
            shader::defs::v_world_normal(),
            "normalize(transpose(inverse(mat3(mat_model))) * rot_normal)",
        )
        .with_out(
            shader::defs::v_world_pos(),
            "mat_model * vec4(scaled_pos, 1.0)",
        )
        .with_out_expr(
            shader::defs::V_POSITION,
            "mat_projection * mat_view * v_world_pos",
        );

    let fragment = shader::FragmentCore::empty()
        .with_in_def(v_discard())
        .with_in_def(v_color())
        .with_body(
            "
            if (v_discard >= 0.5)
                discard;
        ",
        )
        .with_out(shader::defs::f_color(), "v_color");

    shader::Core { vertex, fragment }
}
