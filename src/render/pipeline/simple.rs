use glium::uniforms::UniformType;

use crate::render::object::Vertex;
use crate::render::pipeline::{Context, DefaultInstanceParams, InstanceParamsPair};
use crate::render::shader::{self, VariableQualifier};

type Params = InstanceParamsPair<Context, DefaultInstanceParams>;

pub fn diffuse_shared_variables() -> Vec<shader::SharedVariableDef> {
    vec![
        (
            shader::V_WORLD_NORMAL.into(),
            UniformType::FloatVec3,
            VariableQualifier::Smooth,
        ),
        (
            shader::V_WORLD_POS.into(),
            UniformType::FloatVec4,
            VariableQualifier::Smooth,
        ),
        (
            shader::V_COLOR.into(),
            UniformType::FloatVec4,
            VariableQualifier::Smooth,
        ),
    ]
}

pub fn diffuse_vertex_core() -> shader::VertexCore<Params, Vertex> {
    shader::VertexCore {
        output_defs: diffuse_shared_variables(),
        output_exprs: shader_output_exprs! {
            shader::V_WORLD_NORMAL => "mat3(mat_model) * normal",
            shader::V_WORLD_POS => "mat_model * vec4(position, 1.0)",
            shader::V_COLOR => "color",
            shader::V_POSITION => "mat_projection * mat_view * v_world_pos",
        },
        ..Default::default()
    }
}

pub fn diffuse_fragment_core() -> shader::FragmentCore<Params> {
    shader::FragmentCore {
        input_defs: diffuse_shared_variables(),
        body: "
            float ambient = 0.3;
            float diffuse = max(
                dot(
                    normalize(v_world_normal),
                    normalize(light_pos - v_world_pos.xyz)
                ),
                0.05
            );
        "
        .into(),
        outputs: shader_output_exprs! {
            "f_color", UniformType::FloatVec4 => "(ambient + diffuse) * v_color",
        },
        ..Default::default()
    }
}

/*

        info!("Creating plain render program");
        let plain_program = program!(facade,
            140 => {
                vertex: "
                    #version 140

                    uniform mat4 mat_model;
                    uniform mat4 mat_view;
                    uniform mat4 mat_projection;

                    uniform vec4 color;

                    in vec3 position;
                    out vec4 v_color;

                    void main() {
                        gl_Position = mat_projection
                            * mat_view
                            * mat_model
                            * vec4(position, 1.0);

                        v_color = color;
                    }
                ",

                fragment: "
                    #version 140

                    uniform float t;

                    in vec4 v_color;
                    out vec4 f_color;

                    void main() {
                        f_color = v_color;
                    }
                "
            },
        )?;
*/
